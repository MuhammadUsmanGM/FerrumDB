mod cli;
mod error;
mod metrics;
mod storage;

use rustyline::DefaultEditor;
use std::sync::Arc;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use crate::cli::{parse, print_help, Command};
use crate::metrics::Metrics;
use crate::storage::StorageEngine;

const BANNER: &str = r#"
  ___                          ___  ___
 | __| ___  _ _  _ _  _  _ _ _| _ \| _ )
 | _| / -_)| '_|| '_|| || | '  \  _/| _ \
 |_|  \___||_|  |_|   \_,_|_|_|_|  |___/

  FerrumDB v0.1.0 — Terminal Key-Value Store
  Type HELP for commands, EXIT to quit.
"#;

const DATA_FILE: &str = "ferrumdb.json";

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("ferrumdb=info")),
        )
        .with_target(false)
        .init();

    println!("{BANNER}");

    let engine = match StorageEngine::new(DATA_FILE).await {
        Ok(e) => Arc::new(e),
        Err(e) => {
            error!("Failed to initialize storage: {e}");
            std::process::exit(1);
        }
    };

    let metrics = Arc::new(Metrics::new());
    let mut rl = DefaultEditor::new().expect("Failed to initialize line editor");

    loop {
        let line = match rl.readline("ferrumdb> ") {
            Ok(line) => line,
            Err(
                rustyline::error::ReadlineError::Interrupted
                | rustyline::error::ReadlineError::Eof,
            ) => {
                println!("Goodbye!");
                break;
            }
            Err(e) => {
                error!("Input error: {e}");
                break;
            }
        };

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let _ = rl.add_history_entry(line);

        let cmd = match parse(line) {
            Ok(cmd) => cmd,
            Err(e) => {
                metrics.record_error();
                println!("Error: {e}");
                continue;
            }
        };

        match cmd {
            Command::Set { key, value } => {
                metrics.record_set();
                match engine.set(key, value).await {
                    Ok(Some(_)) => println!("OK (updated)"),
                    Ok(None) => println!("OK"),
                    Err(e) => {
                        metrics.record_error();
                        println!("Error: {e}");
                    }
                }
            }
            Command::Get { key } => {
                metrics.record_get();
                match engine.get(&key).await {
                    Some(val) => println!("{val}"),
                    None => println!("(nil)"),
                }
            }
            Command::Delete { key } => {
                metrics.record_delete();
                match engine.delete(&key).await {
                    Ok(Some(_)) => println!("OK (deleted)"),
                    Ok(None) => println!("(nil) key not found"),
                    Err(e) => {
                        metrics.record_error();
                        println!("Error: {e}");
                    }
                }
            }
            Command::Keys => {
                let keys = engine.keys().await;
                if keys.is_empty() {
                    println!("(empty)");
                } else {
                    for k in &keys {
                        println!("  {k}");
                    }
                    println!("({} keys)", keys.len());
                }
            }
            Command::Count => {
                println!("{} entries", engine.len().await);
            }
            Command::Help => {
                print_help();
                println!("{}", metrics.summary());
            }
            Command::Exit => {
                info!("Shutting down. {}", metrics.summary());
                println!("Goodbye!");
                break;
            }
        }
    }
}
