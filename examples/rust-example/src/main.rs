/// Task Manager — A small FerrumDB example in Rust.
///
/// Demonstrates: CRUD, secondary indexes, transactions, and TTL.
///
/// Run: cargo run

use ferrumdb::StorageEngine;
use ferrumdb::storage::Transaction;
use serde_json::json;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = StorageEngine::new("tasks.db").await?;

    println!("=== FerrumDB Task Manager ===\n");

    // ── Add tasks ──────────────────────────────────────────────
    db.set("task:1".into(), json!({
        "title": "Write README",
        "status": "done",
        "priority": "high"
    })).await?;

    db.set("task:2".into(), json!({
        "title": "Fix login bug",
        "status": "in-progress",
        "priority": "high"
    })).await?;

    db.set("task:3".into(), json!({
        "title": "Add dark mode",
        "status": "todo",
        "priority": "low"
    })).await?;

    db.set("task:4".into(), json!({
        "title": "Update dependencies",
        "status": "todo",
        "priority": "medium"
    })).await?;

    println!("Added {} tasks\n", db.len().await);

    // ── Read a task ────────────────────────────────────────────
    if let Some(task) = db.get("task:2").await {
        println!("Task 2: {}\n", task);
    }

    // ── Secondary indexes: query by status ─────────────────────
    db.create_index("status").await?;
    db.create_index("priority").await?;

    let todo_keys = db.get_by_index("status", &json!("todo")).await;
    println!("TODO tasks: {:?}", todo_keys);

    let high_keys = db.get_by_index("priority", &json!("high")).await;
    println!("High priority: {:?}\n", high_keys);

    // ── Transactions: bulk-add tasks atomically ────────────────
    let tx = Transaction::new()
        .set("task:5".into(), json!({
            "title": "Write tests",
            "status": "todo",
            "priority": "high"
        }))
        .set("task:6".into(), json!({
            "title": "Deploy to prod",
            "status": "todo",
            "priority": "high"
        }));

    db.commit_transaction(tx.build()).await?;
    println!("After transaction: {} tasks total", db.len().await);

    let high_after = db.get_by_index("priority", &json!("high")).await;
    println!("High priority now: {:?}\n", high_after);

    // ── TTL: temporary reminder that expires ───────────────────
    db.set_ex(
        "reminder".into(),
        json!("Stand up and stretch!"),
        Some(Duration::from_secs(2)),
    ).await?;

    println!("Reminder set: {:?}", db.get("reminder").await);
    println!("Waiting 3 seconds for it to expire...");
    tokio::time::sleep(Duration::from_secs(3)).await;
    println!("Reminder after expiry: {:?}\n", db.get("reminder").await);

    // ── Delete a task ──────────────────────────────────────────
    db.delete("task:3").await?;
    println!("Deleted task:3");
    println!("Final count: {} tasks", db.len().await);

    // ── List all keys ──────────────────────────────────────────
    let mut keys = db.keys().await;
    keys.sort();
    println!("All keys: {:?}", keys);

    // Cleanup
    std::fs::remove_file("tasks.db").ok();

    Ok(())
}
