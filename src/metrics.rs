use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use serde::Serialize;

/// Tracks operation counts for observability.
pub struct Metrics {
    gets: AtomicU64,
    sets: AtomicU64,
    deletes: AtomicU64,
    errors: AtomicU64,
    start_time: Instant,
}

/// Serializable metrics snapshot for API responses.
#[derive(Serialize, Debug, Clone)]
pub struct MetricsSnapshot {
    pub uptime_seconds: f64,
    pub total_gets: u64,
    pub total_sets: u64,
    pub total_deletes: u64,
    pub total_errors: u64,
    pub operations_per_second: f64,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            gets: AtomicU64::new(0),
            sets: AtomicU64::new(0),
            deletes: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    pub fn record_get(&self) { self.gets.fetch_add(1, Ordering::Relaxed); }
    pub fn record_set(&self) { self.sets.fetch_add(1, Ordering::Relaxed); }
    pub fn record_delete(&self) { self.deletes.fetch_add(1, Ordering::Relaxed); }
    pub fn record_error(&self) { self.errors.fetch_add(1, Ordering::Relaxed); }

    pub fn summary(&self) -> String {
        let uptime = self.start_time.elapsed();
        format!(
            "Uptime: {:.1}s | GETs: {} | SETs: {} | DELETEs: {} | Errors: {}",
            uptime.as_secs_f64(),
            self.gets.load(Ordering::Relaxed),
            self.sets.load(Ordering::Relaxed),
            self.deletes.load(Ordering::Relaxed),
            self.errors.load(Ordering::Relaxed),
        )
    }

    /// Get a serializable snapshot of current metrics.
    pub fn snapshot(&self) -> MetricsSnapshot {
        let uptime = self.start_time.elapsed().as_secs_f64();
        let total_ops = self.gets.load(Ordering::Relaxed)
            + self.sets.load(Ordering::Relaxed)
            + self.deletes.load(Ordering::Relaxed);
        let ops_per_second = if uptime > 0.0 { total_ops as f64 / uptime } else { 0.0 };

        MetricsSnapshot {
            uptime_seconds: uptime,
            total_gets: self.gets.load(Ordering::Relaxed),
            total_sets: self.sets.load(Ordering::Relaxed),
            total_deletes: self.deletes.load(Ordering::Relaxed),
            total_errors: self.errors.load(Ordering::Relaxed),
            operations_per_second: ops_per_second,
        }
    }
}
