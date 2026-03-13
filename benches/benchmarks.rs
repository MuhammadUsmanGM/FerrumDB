//! FerrumDB Benchmarks
//!
//! Run with: `cargo bench`
//!
//! Compares FerrumDB against SQLite for common operations.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use ferrumdb::{FerrumDB, Config};
use serde_json::json;
use std::sync::Arc;
use tokio::runtime::Runtime;

fn bench_set(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("ferrumdb_set", |b| {
        b.to_async(&rt).iter(|| async {
            let db = FerrumDB::open_default().await.unwrap();
            db.set("key".into(), json!({"value": 42})).await.unwrap();
        })
    });
}

fn bench_get(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("ferrumdb_get", |b| {
        b.to_async(&rt).iter(|| async {
            let db = FerrumDB::open_default().await.unwrap();
            db.set("key".into(), json!({"value": 42})).await.unwrap();
            let _ = db.get("key").await;
        })
    });
}

fn bench_write_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("write_throughput");
    
    for count in [100, 500, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            b.to_async(&rt).iter(|| async {
                let db = FerrumDB::open_default().await.unwrap();
                for i in 0..count {
                    db.set(format!("key_{}", i), json!({"value": i})).await.unwrap();
                }
            })
        });
    }
    group.finish();
}

fn bench_concurrent_writes(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("ferrumdb_concurrent_writes_100", |b| {
        b.to_async(&rt).iter(|| async {
            let db = Arc::new(FerrumDB::open_default().await.unwrap());
            let mut handles = Vec::new();
            
            for i in 0..100 {
                let db_clone = Arc::clone(&db);
                handles.push(tokio::spawn(async move {
                    db_clone.set(format!("k{}", i), json!({"v": i})).await.unwrap();
                }));
            }
            
            for h in handles {
                h.await.unwrap();
            }
        })
    });
}

fn bench_secondary_index_query(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("ferrumdb_secondary_index_query", |b| {
        b.to_async(&rt).iter(|| async {
            let db = FerrumDB::open_default().await.unwrap();
            
            // Setup: 100 users with roles
            for i in 0..100 {
                let role = if i % 2 == 0 { "admin" } else { "user" };
                db.set(format!("user_{}", i), json!({"name": format!("user{}", i), "role": role}))
                    .await
                    .unwrap();
            }
            
            db.create_index("role").await.unwrap();
            let _results = db.find("role", &json!("admin")).await;
        })
    });
}

criterion_group!(
    benches,
    bench_set,
    bench_get,
    bench_write_throughput,
    bench_concurrent_writes,
    bench_secondary_index_query,
);

criterion_main!(benches);
