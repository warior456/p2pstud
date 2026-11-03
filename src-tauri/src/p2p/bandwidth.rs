// src-tauri/src/p2p/bandwidth.rs
// Tracks real-time upload and download bandwidth (bytes per second).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;

/// Thread-safe tracker measuring instantaneous network throughput.
#[derive(Debug)]
pub struct BandwidthTracker {
    bytes_uploaded: AtomicU64,
    bytes_downloaded: AtomicU64,
    last_tick: Mutex<(Instant, u64, u64)>,
    cached_rates: Mutex<(u64, u64)>,
}

impl Default for BandwidthTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl BandwidthTracker {
    pub fn new() -> Self {
        Self {
            bytes_uploaded: AtomicU64::new(0),
            bytes_downloaded: AtomicU64::new(0),
            last_tick: Mutex::new((Instant::now(), 0, 0)),
            cached_rates: Mutex::new((0, 0)),
        }
    }

    /// Record outbound bytes (e.g. uploaded or seeded to peers).
    pub fn record_upload(&self, bytes: u64) {
        self.bytes_uploaded.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record inbound bytes (e.g. downloaded or synced from peers).
    pub fn record_download(&self, bytes: u64) {
        self.bytes_downloaded.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Calculate instantaneous (upload_bytes_per_sec, download_bytes_per_sec)
    /// using a sliding window.
    pub fn get_rates(&self) -> (u64, u64) {
        let mut tick_guard = self.last_tick.lock().unwrap();
        let now = Instant::now();
        let elapsed = now.duration_since(tick_guard.0).as_secs_f64();

        if elapsed >= 1.0 {
            let total_up = self.bytes_uploaded.load(Ordering::Relaxed);
            let total_down = self.bytes_downloaded.load(Ordering::Relaxed);

            let diff_up = total_up.saturating_sub(tick_guard.1);
            let diff_down = total_down.saturating_sub(tick_guard.2);

            let up_bps = (diff_up as f64 / elapsed).round() as u64;
            let down_bps = (diff_down as f64 / elapsed).round() as u64;

            let mut cached_guard = self.cached_rates.lock().unwrap();
            *cached_guard = (up_bps, down_bps);

            *tick_guard = (now, total_up, total_down);
            (up_bps, down_bps)
        } else {
            *self.cached_rates.lock().unwrap()
        }
    }
}

