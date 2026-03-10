use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Tracks operation counts for observability.
pub struct Metrics {
    gets: AtomicU64,
    sets: AtomicU64,
    deletes: AtomicU64,
    errors: AtomicU64,
    start_time: Instant,
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
}
