use axum::response::IntoResponse;
use clap::{Parser, Subcommand};
use ferrumdb::StorageEngine;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "ferrumdb", about = "FerrumDB CLI — manage your database from the terminal")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Launch Ferrum Studio web dashboard
    Web {
        /// Path to the .db file
        path: String,

        /// Port to serve the dashboard on
        #[arg(short, long, default_value_t = 7474)]
        port: u16,
    },

    /// Show database info (key count, file size)
    Info {
        /// Path to the .db file
        path: String,
    },

    /// Compact the database (remove deleted/expired entries)
    Compact {
        /// Path to the .db file
        path: String,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Web { path, port } => {
            let engine = match StorageEngine::new(&path).await {
                Ok(e) => Arc::new(e),
                Err(e) => {
                    eprintln!("Error opening database '{}': {}", path, e);
                    std::process::exit(1);
                }
            };

            let key_count = engine.len().await;
            println!();
            println!("  \x1b[38;5;208m⚡ FerrumDB Studio\x1b[0m");
            println!("  Database: {}", path);
            println!("  Keys:     {}", key_count);
            println!();
            println!("  \x1b[38;5;208m🔥 http://localhost:{}\x1b[0m", port);
            println!();
            println!("  Press Ctrl+C to stop.");
            println!();

            // Start studio in foreground (not spawned as background task)
            let app = axum::Router::new()
                .route("/", axum::routing::get(|| async {
                    axum::response::Html(ferrumdb::studio::DASHBOARD_HTML)
                }))
                .route("/api/keys", axum::routing::get({
                    let eng = Arc::clone(&engine);
                    move || async move {
                        let keys = eng.keys().await;
                        axum::Json(serde_json::json!({ "keys": keys, "count": keys.len() }))
                    }
                }))
                .route("/api/count", axum::routing::get({
                    let eng = Arc::clone(&engine);
                    move || async move {
                        axum::Json(serde_json::json!({ "count": eng.len().await }))
                    }
                }))
                .route("/api/get/{key}", axum::routing::get({
                    let eng = Arc::clone(&engine);
                    move |axum::extract::Path(key): axum::extract::Path<String>| async move {
                        match eng.get(&key).await {
                            Some(val) => (axum::http::StatusCode::OK, axum::Json(serde_json::json!({ "key": key, "value": val }))).into_response(),
                            None => (axum::http::StatusCode::NOT_FOUND, axum::Json(serde_json::json!({ "error": "Key not found" }))).into_response(),
                        }
                    }
                }))
                .route("/api/set", axum::routing::post({
                    let eng = Arc::clone(&engine);
                    move |axum::Json(body): axum::Json<serde_json::Value>| async move {
                        let key = body.get("key").and_then(|k| k.as_str()).unwrap_or("").to_string();
                        let value = body.get("value").cloned().unwrap_or(serde_json::Value::Null);
                        if key.is_empty() {
                            return (axum::http::StatusCode::BAD_REQUEST, axum::Json(serde_json::json!({ "error": "Missing key" }))).into_response();
                        }
                        match eng.set(key.clone(), value).await {
                            Ok(_) => (axum::http::StatusCode::OK, axum::Json(serde_json::json!({ "ok": true, "key": key }))).into_response(),
                            Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, axum::Json(serde_json::json!({ "error": e.to_string() }))).into_response(),
                        }
                    }
                }))
                .route("/api/delete/{key}", axum::routing::delete({
                    let eng = Arc::clone(&engine);
                    move |axum::extract::Path(key): axum::extract::Path<String>| async move {
                        match eng.delete(&key).await {
                            Ok(Some(_)) => (axum::http::StatusCode::OK, axum::Json(serde_json::json!({ "ok": true, "key": key }))).into_response(),
                            Ok(None) => (axum::http::StatusCode::NOT_FOUND, axum::Json(serde_json::json!({ "error": "Key not found" }))).into_response(),
                            Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, axum::Json(serde_json::json!({ "error": e.to_string() }))).into_response(),
                        }
                    }
                }))
                .route("/api/metrics", axum::routing::get({
                    let eng = Arc::clone(&engine);
                    move || async move {
                        axum::Json(serde_json::json!(eng.metrics().snapshot()))
                    }
                }));

            let addr = format!("0.0.0.0:{}", port);
            let listener = tokio::net::TcpListener::bind(&addr).await.unwrap_or_else(|e| {
                eprintln!("Error binding to port {}: {}", port, e);
                std::process::exit(1);
            });

            axum::serve(listener, app).await.unwrap_or_else(|e| {
                eprintln!("Server error: {}", e);
                std::process::exit(1);
            });
        }

        Commands::Info { path } => {
            let engine = match StorageEngine::new(&path).await {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("Error opening database '{}': {}", path, e);
                    std::process::exit(1);
                }
            };

            let file_size = std::fs::metadata(&path)
                .map(|m| m.len())
                .unwrap_or(0);

            println!();
            println!("  \x1b[38;5;208m⚡ FerrumDB Info\x1b[0m");
            println!("  Path:      {}", path);
            println!("  Keys:      {}", engine.len().await);
            println!("  File size: {}", format_bytes(file_size));
            println!();
        }

        Commands::Compact { path } => {
            let engine = match StorageEngine::new(&path).await {
                Ok(e) => Arc::new(e),
                Err(e) => {
                    eprintln!("Error opening database '{}': {}", path, e);
                    std::process::exit(1);
                }
            };

            let before = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            println!("  Compacting {}...", path);

            match engine.compact(&path).await {
                Ok(_) => {
                    let after = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                    println!("  Done. {} -> {} (saved {})",
                        format_bytes(before),
                        format_bytes(after),
                        format_bytes(before.saturating_sub(after)),
                    );
                }
                Err(e) => {
                    eprintln!("  Compaction failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
