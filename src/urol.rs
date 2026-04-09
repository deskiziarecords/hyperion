// ─────────────────────────────────────────────────────────────────────────────
// urol.rs  –  Universal Reliability & Observability Layer (UROL)
//
// Responsibilities:
//   • Ingest raw ticks from an upstream feed (stub here).
//   • Apply a MAD (Median Absolute Deviation) outlier filter.
//   • Bucket ticks into fixed OHLCV bars and publish to `clean:ticks`.
//   • Run a watchdog that monitors heartbeats and triggers a kill-switch
//     if the process stalls or drawdown exceeds the Mandra-gate threshold.
// ─────────────────────────────────────────────────────────────────────────────

use anyhow::{Context, Result};
use redis::{aio::MultiplexedConnection, AsyncCommands};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::{interval, sleep};
use tracing::{error, info, warn};

use crate::{
    config::{BUCKET_MS, STREAM_CLEAN_TICKS, STATE_KEY},
    state::SharedState,
    types::Bar,
};

// ── Config ────────────────────────────────────────────────────────────────────

/// MAD filter: reject ticks whose deviation from the rolling median exceeds
/// this many MADs.
const MAD_THRESHOLD: f64 = 3.5;

/// Rolling window size for the MAD filter (ticks).
const MAD_WINDOW: usize = 50;

/// Drawdown level at which the watchdog halts trading (Mandra gate level 3).
const MANDRA_GATE_DRAWDOWN: f64 = 0.08; // 8 %

/// How often the watchdog checks heartbeat + drawdown (ms).
const WATCHDOG_INTERVAL_MS: u64 = 5_000;

/// Maximum time (ms) allowed between ticks before watchdog fires.
const HEARTBEAT_TIMEOUT_MS: u64 = 30_000;

// ── Raw tick (from upstream feed) ─────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RawTick {
    pub price:  f64,
    pub volume: f64,
    pub ts_ms:  i64,
}

// ── OHLCV bucket accumulator ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct Bucket {
    open:   f64,
    high:   f64,
    low:    f64,
    close:  f64,
    volume: f64,
    start_ms: i64,
}

impl Bucket {
    fn new(tick: &RawTick) -> Self {
        Self {
            open:     tick.price,
            high:     tick.price,
            low:      tick.price,
            close:    tick.price,
            volume:   tick.volume,
            start_ms: (tick.ts_ms / BUCKET_MS) * BUCKET_MS,
        }
    }

    fn update(&mut self, tick: &RawTick) {
        self.high   = self.high.max(tick.price);
        self.low    = self.low.min(tick.price);
        self.close  = tick.price;
        self.volume += tick.volume;
    }

    fn to_bar(&self) -> Bar {
        Bar {
            open:   self.open,
            high:   self.high,
            low:    self.low,
            close:  self.close,
            volume: self.volume,
            ts:     self.start_ms,
        }
    }
}

// ── MAD filter ────────────────────────────────────────────────────────────────

struct MadFilter {
    window: Vec<f64>,
}

impl MadFilter {
    fn new() -> Self { Self { window: Vec::with_capacity(MAD_WINDOW + 1) } }

    /// Returns `true` if the tick price passes the MAD filter (is not an outlier).
    fn accept(&mut self, price: f64) -> bool {
        // Need enough data first
        if self.window.len() < 10 {
            self.window.push(price);
            return true;
        }

        let median = self.median();
        let mad    = self.mad(median);

        // Accept if within MAD_THRESHOLD × MAD of median
        let ok = mad == 0.0 || ((price - median).abs() / mad) <= MAD_THRESHOLD;

        self.window.push(price);
        if self.window.len() > MAD_WINDOW {
            self.window.remove(0);
        }
        ok
    }

    fn median(&self) -> f64 {
        let mut sorted = self.window.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = sorted.len();
        if n % 2 == 0 {
            (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
        } else {
            sorted[n / 2]
        }
    }

    fn mad(&self, median: f64) -> f64 {
        let mut deviations: Vec<f64> = self.window.iter().map(|&x| (x - median).abs()).collect();
        deviations.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = deviations.len();
        if n % 2 == 0 {
            (deviations[n / 2 - 1] + deviations[n / 2]) / 2.0
        } else {
            deviations[n / 2]
        }
    }
}

// ── Public entry-points ───────────────────────────────────────────────────────

/// UROL ingestion loop: tick feed → MAD filter → OHLCV bucketing → `clean:ticks`.
///
/// In production, replace `tick_feed()` with your real data-feed adapter
/// (WebSocket, FIX, REST polling, etc.).
pub async fn run_ingestion(mut con: MultiplexedConnection, shared_state: SharedState) -> Result<()> {
    let mut mad       = MadFilter::new();
    let mut bucket: Option<Bucket> = None;

    info!("UROL ingestion started");

    loop {
        // ── Pull next raw tick (replace stub with real feed) ──────────────
        let tick = tick_feed_stub().await;
        let bucket_slot = (tick.ts_ms / BUCKET_MS) * BUCKET_MS;

        // ── MAD filter ────────────────────────────────────────────────────
        if !mad.accept(tick.price) {
            warn!(price = tick.price, "MAD filter rejected tick");
            continue;
        }

        // ── Bucket accumulation / flush ───────────────────────────────────
        match bucket.as_mut() {
            Some(b) if b.start_ms == bucket_slot => {
                b.update(&tick);
            }
            _ => {
                // Flush completed bucket
                if let Some(completed) = bucket.take() {
                    let bar = completed.to_bar();
                    publish_bar(&mut con, &bar).await?;

                    // Update last_fft_spectrum placeholder in shared state
                    let mut state = shared_state.write().await;
                    state.last_fft_spectrum = vec![bar.close]; // replace with real FFT
                }
                bucket = Some(Bucket::new(&tick));
            }
        }
    }
}

/// UROL watchdog: monitors heartbeat and drawdown; fires kill-switch if stale.
pub async fn run_watchdog(mut con: MultiplexedConnection, shared_state: SharedState) -> Result<()> {
    let mut tick = interval(Duration::from_millis(WATCHDOG_INTERVAL_MS));
    let mut last_heartbeat_ms = unix_ms();

    info!("UROL watchdog started");

    loop {
        tick.tick().await;

        let now = unix_ms();
        let state = shared_state.read().await.clone();

        // ── Heartbeat check ───────────────────────────────────────────────
        // In production, the ingestion loop would write to a heartbeat key.
        // Here we approximate by checking Redis key age.
        let hb_age: Option<i64> = con.pttl(STATE_KEY).await.ok();
        let stale = match hb_age {
            Some(age) if age > 0 => (now - last_heartbeat_ms) > HEARTBEAT_TIMEOUT_MS as i64,
            _ => false,
        };
        if !stale {
            last_heartbeat_ms = now;
        }

        if stale {
            error!("UROL watchdog: heartbeat timeout – triggering kill-switch");
            trigger_kill_switch(&mut con, &shared_state).await;
            continue;
        }

        // ── Mandra-gate drawdown check ────────────────────────────────────
        if state.current_drawdown >= MANDRA_GATE_DRAWDOWN {
            error!(
                drawdown = state.current_drawdown,
                threshold = MANDRA_GATE_DRAWDOWN,
                "UROL watchdog: Mandra-gate triggered"
            );
            trigger_kill_switch(&mut con, &shared_state).await;
            continue;
        }

        info!(
            phase    = %state.ipda_phase,
            drawdown = state.current_drawdown,
            kz       = state.kill_zone_active,
            "Watchdog OK"
        );
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Publish a completed OHLCV bar to `clean:ticks`.
async fn publish_bar(con: &mut MultiplexedConnection, bar: &Bar) -> Result<()> {
    let payload = serde_json::to_string(bar).context("Bar serialise")?;
    let _: String = con
        .xadd(STREAM_CLEAN_TICKS, "*", &[("payload", payload)])
        .await
        .context("xadd clean:ticks")?;
    Ok(())
}

/// Kill-switch: set Mandra gate level 3, close all positions flag in state.
async fn trigger_kill_switch(con: &mut MultiplexedConnection, shared_state: &SharedState) {
    let mut state = shared_state.write().await;
    state.mandra_gate_level = 3;
    state.open_positions.clear(); // signal to execution layer to flatten

    let json = serde_json::to_string(&*state).unwrap_or_default();
    let _: Result<(), _> = con.set::<_, _, ()>(STATE_KEY, json).await;

    // Also publish a FLAT signal directly to jax:signals so AECABI closes positions
    let flat = crate::types::Signal {
        action:    crate::types::Action::Flat,
        size:      0.0,
        phase:     crate::types::Phase::Flat,
        kill_zone: false,
        timestamp: unix_now(),
        price:     None,
    };
    if let Ok(payload) = serde_json::to_string(&flat) {
        let _: Result<String, _> = con
            .xadd(crate::config::STREAM_JAX_SIGNALS, "*", &[("payload", payload)])
            .await;
    }

    warn!("Kill-switch fired: Mandra gate level 3 – all positions flagged for closure");
}

/// Tick-feed stub — replace with your real WebSocket / REST feed adapter.
async fn tick_feed_stub() -> RawTick {
    sleep(Duration::from_millis(100)).await; // simulate ~10 ticks/sec

    // Simulate a slow random walk around 1.1000
    static PRICE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(11000);
    let current = PRICE.load(std::sync::atomic::Ordering::Relaxed);
    // ±1 pip random walk
    let delta: i64 = if unix_ms() % 2 == 0 { 1 } else { -1 };
    let next = (current as i64 + delta).max(10000) as u64;
    PRICE.store(next, std::sync::atomic::Ordering::Relaxed);

    RawTick {
        price:  next as f64 / 10_000.0,
        volume: 1.0 + (unix_ms() % 10) as f64,
        ts_ms:  unix_ms(),
    }
}

fn unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn unix_now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}
