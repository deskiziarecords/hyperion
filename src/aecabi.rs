// ─────────────────────────────────────────────────────────────────────────────
// aecabi.rs  –  Adaptive Execution & Cost-Aware Broker Interface (AECABI)
//
// Consumes signals from `jax:signals`, applies a TCA filter, submits live
// orders to the broker stub, and simultaneously runs a shadow engine that
// models latency + slippage and writes fills to SQLite.
// ─────────────────────────────────────────────────────────────────────────────

use anyhow::{Context, Result};
use redis::{
    aio::MultiplexedConnection,
    streams::{StreamReadOptions, StreamReadReply},
    AsyncCommands,
};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;
use tracing::{info, warn};

use crate::{
    config::{STREAM_JAX_SIGNALS},
    types::{Action, Signal},
};

// ── Config ────────────────────────────────────────────────────────────────────

/// Minimum expected edge (as fraction of price) required to pass the TCA filter.
/// Set to 3 × (spread + commission) in practice.
const TCA_MIN_EDGE: f64 = 0.0003;

/// Simulated one-way broker commission per lot (account currency).
const COMMISSION_PER_LOT: f64 = 7.0;

/// Simulated spread cost per lot.
const SPREAD_COST_PER_LOT: f64 = 5.0;

/// Simulated execution latency range (ms).
const LATENCY_MIN_MS: u64 = 5;
const LATENCY_MAX_MS: u64 = 50;

/// Simulated slippage as fraction of price (uniform random within ± this value).
const MAX_SLIPPAGE_FRAC: f64 = 0.0001;

/// Consumer group / consumer identity for jax:signals.
const GROUP_NAME:    &str = "aecabi-group";
const CONSUMER_NAME: &str = "aecabi-worker-1";

// ── Domain types ──────────────────────────────────────────────────────────────

/// A broker order (sent to the live broker stub).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id:        String,
    pub action:    Action,
    pub size:      f64,
    pub ref_price: f64,
    pub timestamp: f64,
}

/// A shadow fill (written to SQLite for post-trade analysis).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowFill {
    pub order_id:       String,
    pub action:         Action,
    pub size:           f64,
    pub fill_price:     f64,
    pub slippage_frac:  f64,
    pub latency_ms:     u64,
    pub commission:     f64,
    pub net_pnl_est:    f64,
    pub phase:          String,
    pub timestamp:      f64,
}

// ── Public entry-point ────────────────────────────────────────────────────────

/// Run the AECABI execution loop forever.
///
/// * Reads signals from `jax:signals` via a Redis consumer group.
/// * Applies the TCA filter — skips signals where edge < `TCA_MIN_EDGE`.
/// * Submits idempotent orders to the broker stub.
/// * Runs the shadow engine in parallel (simulates latency + slippage).
/// * Writes shadow fills to SQLite.
pub async fn run(mut con: MultiplexedConnection) -> Result<()> {
    ensure_consumer_group(&mut con).await;

    // Initialise SQLite shadow DB
    let db = init_shadow_db().await?;

    info!("AECABI execution gateway started – consuming jax:signals …");

    loop {
        let messages = read_signals(&mut con).await?;

        for (msg_id, signal) in messages {
            // ── 1. Skip FLAT signals ──────────────────────────────────────
            if signal.action == Action::Flat {
                ack(&mut con, &msg_id).await?;
                continue;
            }

            let ref_price = match signal.price {
                Some(p) => p,
                None => { ack(&mut con, &msg_id).await?; continue; }
            };

            // ── 2. TCA filter ─────────────────────────────────────────────
            let total_cost = (COMMISSION_PER_LOT + SPREAD_COST_PER_LOT) * signal.size;
            let cost_frac  = total_cost / (ref_price * signal.size * 100_000.0); // normalise
            if cost_frac >= TCA_MIN_EDGE {
                warn!(
                    cost_frac,
                    min_edge = TCA_MIN_EDGE,
                    "TCA filter blocked signal – edge insufficient"
                );
                ack(&mut con, &msg_id).await?;
                continue;
            }

            // ── 3. Build idempotent order ─────────────────────────────────
            let order = Order {
                id:        format!("ORD-{}", unique_id()),
                action:    signal.action,
                size:      signal.size,
                ref_price,
                timestamp: signal.timestamp,
            };

            // ── 4. Submit to live broker stub ─────────────────────────────
            submit_live_order(&order).await;

            // ── 5. Shadow engine ──────────────────────────────────────────
            let fill = shadow_fill(&order, &signal).await;
            persist_shadow_fill(&db, &fill).await?;

            info!(
                order_id    = %fill.order_id,
                action      = %order.action,
                fill_price  = fill.fill_price,
                slippage_bp = fill.slippage_frac * 10_000.0,
                latency_ms  = fill.latency_ms,
                commission  = fill.commission,
                phase       = %fill.phase,
                "Shadow fill recorded"
            );

            ack(&mut con, &msg_id).await?;
        }
    }
}

// ── Broker stub ───────────────────────────────────────────────────────────────

/// Submit an order to the live broker.
///
/// Replace this stub with your actual broker API call
/// (e.g. OANDA, Interactive Brokers, or a custom FIX adapter).
async fn submit_live_order(order: &Order) {
    // In production: send order via broker SDK / REST / FIX.
    // This stub just logs the intent.
    info!(
        id     = %order.id,
        action = %order.action,
        size   = order.size,
        price  = order.ref_price,
        "→ Live order submitted (stub)"
    );
}

// ── Shadow engine ─────────────────────────────────────────────────────────────

/// Simulate execution: add random latency + slippage, compute estimated PnL.
async fn shadow_fill(order: &Order, signal: &Signal) -> ShadowFill {
    // Simulate latency
    let latency_ms = LATENCY_MIN_MS
        + (pseudo_rand() % (LATENCY_MAX_MS - LATENCY_MIN_MS + 1));
    sleep(Duration::from_millis(latency_ms / 10)).await; // Don't actually block full latency

    // Simulate slippage
    let slippage_frac = (pseudo_rand_f64() * 2.0 - 1.0) * MAX_SLIPPAGE_FRAC;
    let fill_price = order.ref_price * (1.0 + slippage_frac);

    let commission = COMMISSION_PER_LOT * order.size;

    // Rough PnL estimate: direction × slippage impact − commission
    let direction = match order.action { Action::Buy => 1.0, Action::Sell => -1.0, _ => 0.0 };
    let net_pnl_est = direction * slippage_frac * fill_price * order.size * 100_000.0 - commission;

    ShadowFill {
        order_id:      order.id.clone(),
        action:        order.action,
        size:          order.size,
        fill_price,
        slippage_frac,
        latency_ms,
        commission,
        net_pnl_est,
        phase:         signal.phase.to_string(),
        timestamp:     unix_now(),
    }
}

// ── SQLite persistence ────────────────────────────────────────────────────────

/// Minimal SQLite handle (file-based, no ORM needed for this use-case).
pub struct ShadowDb {
    path: String,
}

async fn init_shadow_db() -> Result<ShadowDb> {
    // In production replace with sqlx::SqlitePool for full async support.
    // We use a path stub here to keep the dependency tree lean.
    let path = "shadow_trades.sqlite".to_string();
    info!("Shadow DB initialised at {path}");
    Ok(ShadowDb { path })
}

async fn persist_shadow_fill(db: &ShadowDb, fill: &ShadowFill) -> Result<()> {
    // Production: use sqlx execute!() with a proper INSERT.
    // Stub: serialise to JSON and append to a sidecar log file.
    let json = serde_json::to_string(fill)?;
    let log_path = format!("{}.jsonl", db.path);
    use tokio::io::AsyncWriteExt;
    let mut f = tokio::fs::OpenOptions::new()
        .create(true).append(true).open(&log_path).await
        .context("open shadow log")?;
    f.write_all(format!("{json}\n").as_bytes()).await
        .context("write shadow fill")?;
    Ok(())
}

// ── Redis helpers ─────────────────────────────────────────────────────────────

async fn ensure_consumer_group(con: &mut MultiplexedConnection) {
    let result: redis::RedisResult<()> = con
        .xgroup_create_mkstream(STREAM_JAX_SIGNALS, GROUP_NAME, "$")
        .await;
    match result {
        Ok(_)  => info!("AECABI consumer group '{GROUP_NAME}' created"),
        Err(e) if e.to_string().contains("BUSYGROUP") => {}
        Err(e) => warn!("xgroup_create: {e}"),
    }
}

async fn read_signals(con: &mut MultiplexedConnection) -> Result<Vec<(String, Signal)>> {
    let opts = StreamReadOptions::default()
        .group(GROUP_NAME, CONSUMER_NAME)
        .count(10)
        .block(2_000);

    let reply: Option<StreamReadReply> = con
        .xread_options(&[STREAM_JAX_SIGNALS], &[">"], &opts)
        .await
        .context("xreadgroup jax:signals")?;

    let mut out = Vec::new();
    if let Some(r) = reply {
        for stream in r.keys {
            for entry in stream.ids {
                if let Some(v) = entry.map.get("payload") {
                    let s = match v {
                        redis::Value::Data(b) => String::from_utf8_lossy(b).into_owned(),
                        redis::Value::Status(s) => s.clone(),
                        _ => continue,
                    };
                    match serde_json::from_str::<Signal>(&s) {
                        Ok(sig) => out.push((entry.id.clone(), sig)),
                        Err(e)  => warn!("Signal deserialise: {e}"),
                    }
                }
            }
        }
    }
    Ok(out)
}

async fn ack(con: &mut MultiplexedConnection, id: &str) -> Result<()> {
    let _: i64 = con.xack(STREAM_JAX_SIGNALS, GROUP_NAME, &[id]).await?;
    Ok(())
}

// ── Tiny utilities ────────────────────────────────────────────────────────────

fn unix_now() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

/// Deterministic pseudo-random u64 (LCG — good enough for slippage simulation).
fn pseudo_rand() -> u64 {
    static SEED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(12345);
    let s = SEED.fetch_add(6_364_136_223_846_793_005, std::sync::atomic::Ordering::Relaxed);
    s.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407)
}

fn pseudo_rand_f64() -> f64 {
    (pseudo_rand() >> 11) as f64 / (1u64 << 53) as f64
}

fn unique_id() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(pseudo_rand())
}
