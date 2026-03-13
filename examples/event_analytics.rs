//! Real-world use case: Event Analytics System
//!
//! This demonstrates FerrumDB powering a local event tracking system
//! that ingests user events and provides analytics queries.
//!
//! Scenario: Track user actions (page views, clicks, purchases) and query by:
//! - User ID
//! - Event type
//! - Time range (via TTL)
//! - Properties (via secondary indexes)

use ferrumdb::{FerrumDB, Transaction};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== FerrumDB: Event Analytics Demo ===\n");

    // Open database for event storage
    let db = FerrumDB::open_default().await?;

    // 1. Ingest events (simulating a high-throughput event stream)
    println!("1. Ingesting 500 events...");
    let events = generate_sample_events(500);
    
    let start = std::time::Instant::now();
    for event in events {
        db.set(event.id, event.data).await?;
    }
    let ingest_duration = start.elapsed();
    
    println!("   ✓ Ingested 500 events in {:?} ({:.0} events/sec)", 
             ingest_duration, 
             500.0 / ingest_duration.as_secs_f64());

    // 2. Create secondary indexes for analytics queries
    println!("\n2. Creating secondary indexes...");
    db.create_index("event_type").await?;
    db.create_index("user_id").await?;
    db.create_index("page").await?;
    println!("   ✓ Indexes created: event_type, user_id, page");

    // 3. Run analytics queries
    println!("\n3. Running analytics queries...");
    
    // Query: All page_view events
    let page_views = db.find("event_type", &json!("page_view")).await;
    println!("   - Page view events: {}", page_views.len());
    
    // Query: All purchase events
    let purchases = db.find("event_type", &json!("purchase")).await;
    println!("   - Purchase events: {}", purchases.len());
    
    // Query: Events for specific user
    let user_events = db.find("user_id", &json!("user_42")).await;
    println!("   - Events for user_42: {}", user_events.len());
    
    // Query: Events on specific page
    let pricing_events = db.find("page", &json!("/pricing")).await;
    println!("   - Events on /pricing page: {}", pricing_events.len());

    // 4. Demonstrate atomic transaction for batch event ingestion
    println!("\n4. Demonstrating atomic batch ingestion...");
    let batch_tx = Transaction::new()
        .set("event:batch:1".into(), json!({
            "event_type": "click",
            "user_id": "user_100",
            "button": "signup_cta",
            "page": "/landing"
        }))
        .set("event:batch:2".into(), json!({
            "event_type": "click",
            "user_id": "user_101",
            "button": "signup_cta",
            "page": "/landing"
        }))
        .set("event:batch:3".into(), json!({
            "event_type": "conversion",
            "user_id": "user_100",
            "value": 99.00,
            "campaign": "summer_sale"
        }));
    
    db.commit(batch_tx).await?;
    println!("   ✓ Atomically committed 3 events");

    // 5. Show database metrics
    println!("\n5. Database metrics:");
    println!("   - Total events: {}", db.engine().len().await);
    println!("   - Keys: {:?}", db.engine().keys().await);

    // 6. Demonstrate TTL for session-like data
    println!("\n6. Demonstrating TTL for session data...");
    use std::time::Duration;
    db.engine()
        .set_ex(
            "session:abc123".into(),
            json!({"user_id": "user_42", "expires_in": "2 seconds"}),
            Duration::from_secs(2),
        )
        .await?;
    
    println!("   - Session stored with 2-second TTL");
    println!("   - Session exists: {:?}", db.get("session:abc123").await.is_some());
    
    tokio::time::sleep(Duration::from_secs(3)).await;
    println!("   - After expiry, session exists: {:?}", db.get("session:abc123").await.is_some());

    // 7. Demonstrate encrypted storage for sensitive data
    println!("\n7. Demonstrating encrypted storage...");
    let key: [u8; 32] = *b"my_super_secret_key_32_bytes_!!";
    let config = ferrumdb::Config::new()
        .with_encryption(key)
        .with_path("events_encrypted.db".into());
    let encrypted_db = FerrumDB::open(config).await?;
    
    encrypted_db.set(
        "pii:user_ssn".into(),
        json!({"ssn": "123-45-6789", "encrypted": true}),
    ).await?;
    
    println!("   - Stored sensitive data in encrypted database");
    println!("   - Retrieved: {:?}", encrypted_db.get("pii:user_ssn").await);

    println!("\n=== Demo Complete ===");
    println!("\nThis demo shows FerrumDB can handle:");
    println!("  ✓ High-throughput event ingestion (500+ events)");
    println!("  ✓ Secondary index queries for analytics");
    println!("  ✓ Atomic batch operations");
    println!("  ✓ TTL-based expiration for session data");
    println!("  ✓ AES-256 encryption for sensitive data");
    
    Ok(())
}

/// Sample event structure for analytics
struct Event {
    id: String,
    data: serde_json::Value,
}

/// Generate sample events for testing
fn generate_sample_events(count: usize) -> Vec<Event> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    
    let event_types = ["page_view", "click", "purchase", "signup", "logout"];
    let pages = ["/home", "/pricing", "/features", "/dashboard", "/settings", "/landing"];
    let browsers = ["chrome", "firefox", "safari", "edge"];
    
    (0..count)
        .map(|i| {
            let event_type = event_types[rng.gen_range(0..event_types.len())];
            let user_id = format!("user_{}", rng.gen_range(1..100));
            let page = pages[rng.gen_range(0..pages.len())];
            
            let mut data = json!({
                "event_type": event_type,
                "user_id": user_id,
                "page": page,
                "browser": browsers[rng.gen_range(0..browsers.len())],
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });
            
            // Add event-specific properties
            if event_type == "purchase" {
                data["value"] = json!(rng.gen_range(10..500));
                data["currency"] = json!("USD");
            } else if event_type == "click" {
                data["element"] = json!(format!("btn_{}", rng.gen_range(1..20)));
            }
            
            Event {
                id: format!("event:{}", i),
                data,
            }
        })
        .collect()
}
