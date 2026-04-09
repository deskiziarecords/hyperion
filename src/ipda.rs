// ─────────────────────────────────────────────────────────────────────────────
// ipda.rs  –  IPDA Core Signal Producer
//
// Reads clean OHLCV bars from `clean:ticks` (UROL output),
// runs phase detection + kill-zone gating,
// and publishes a typed signal to `jax:signals` (consumed by AECABI).
// ─────────────────────────────────────────────────────────────────────────────

use anyhow::{Context, Result};
use redis::{
    aio::MultiplexedConnection,
    streams::{StreamReadOptions, StreamReadReply},
    AsyncCommands,
};
use std::collections::VecDeque;
use tracing::{debug, info, warn};

use crate::{
    config::{
        ATR_PERIOD, BARS_PER_DAY, EQUITY, LOOKBACK_DAYS, PIP_VALUE, RISK_PER_TRADE,
        STREAM_CLEAN_TICKS, STREAM_JAX_SIGNALS,
    },
    indicators::atr,
    kill_zone::is_kill_zone,
    phase::{detect_phase, MAX_BUFFER_BARS},
    state::SharedState,
    types::{Action, Bar, Phase, Signal},
};

// ── Constants ─────────────────────────────────────────────────────────────────

/// Consumer group + consumer name for the Redis Streams API.
const GROUP_NAME:    &str = "ipda-group";
const CONSUMER_NAME: &str = "ipda-worker-1";

/// How many messages to fetch per XREADGROUP call.
const BATCH_SIZE: usize = 10;

// ── Public entry-point ────────────────────────────────────────────────────────

/// Run the IPDA signal loop forever (call with `tokio::spawn`).
///
/// * Reads bars from `clean:ticks` via a Redis consumer group.
/// * Maintains a rolling `VecDeque<Bar>` of up to `MAX_BUFFER_BARS`.
/// * Detects the IPDA phase, gates on kill-zones, sizes the trade.
/// * Publishes a `Signal` JSON to `jax:signals`.
/// * Updates the shared `GlobalState` after every bar.
pub async fn run(mut con: MultiplexedConnection, shared_state: SharedState) -> Result<()> {
    // Ensure the consumer group exists (MKSTREAM creates the stream if absent)
    ensure_consumer_group(&mut con).await;

    let mut bar_buffer: VecDeque<Bar> = VecDeque::with_capacity(MAX_BUFFER_BARS + 1);

    info!("IPDA core started – waiting for clean:ticks …");

    loop {
        let messages = read_messages(&mut con).await?;

        for (msg_id, bar) in messages {
            // ── 1. Update rolling buffer ──────────────────────────────────
            bar_buffer.push_back(bar.clone());
            if bar_buffer.len() > MAX_BUFFER_BARS {
                bar_buffer.pop_front();
            }

            let bars: Vec<Bar> = bar_buffer.iter().cloned().collect();

            // ── 2. Detect IPDA phase ──────────────────────────────────────
            let phase = detect_phase(&bars);

            // ── 3. Kill-zone gate ─────────────────────────────────────────
            let in_kill_zone = is_kill_zone(bar.ts);

            let signal = if !in_kill_zone {
                // Outside session window → emit FLAT
                Signal {
                    action:    Action::Flat,
                    size:      0.0,
                    phase,
                    kill_zone: false,
                    timestamp: unix_now(),
                    price:     None,
                }
            } else {
                // ── 4. Size & direction ───────────────────────────────────
                let (action, size) = compute_action_size(phase, &bars);
                Signal {
                    action,
                    size,
                    phase,
                    kill_zone: true,
                    timestamp: unix_now(),
                    price: Some(bar.close),
                }
            };

            // ── 5. Publish signal ─────────────────────────────────────────
            publish_signal(&mut con, &signal).await?;

            info!(
                action  = %signal.action,
                size    = signal.size,
                phase   = %signal.phase,
                kz      = signal.kill_zone,
                "IPDA signal"
            );

            // ── 6. Update shared state ────────────────────────────────────
            {
                let mut state = shared_state.write().await;
                state.ipda_phase       = phase;
                state.kill_zone_active = in_kill_zone;
                state.ipda_lookback    = LOOKBACK_DAYS[0];
                if phase == Phase::Accumulation && state.accumulation_start_ts.is_none() {
                    state.accumulation_start_ts = Some(bar.ts / 1_000);
                } else if phase != Phase::Accumulation {
                    state.accumulation_start_ts = None;
                }
            }

            // ── 7. Acknowledge message ────────────────────────────────────
            ack_message(&mut con, &msg_id).await?;
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Compute trade direction and lot size for a given phase.
///
/// `size = (equity × risk%) / (ATR₂₀ × pip_value)`
fn compute_action_size(phase: Phase, bars: &[Bar]) -> (Action, f64) {
    match phase {
        Phase::Accumulation => {
            let atr20 = atr(bars, ATR_PERIOD);
            let size  = if atr20 > 0.0 {
                (EQUITY * RISK_PER_TRADE) / (atr20 * PIP_VALUE)
            } else { 0.0 };
            (Action::Buy, size)
        }
        Phase::Distribution => {
            let atr20 = atr(bars, ATR_PERIOD);
            let size  = if atr20 > 0.0 {
                (EQUITY * RISK_PER_TRADE) / (atr20 * PIP_VALUE)
            } else { 0.0 };
            (Action::Sell, size)
        }
        _ => (Action::Flat, 0.0),
    }
}

/// Publish a `Signal` as a JSON payload to the `jax:signals` Redis stream.
async fn publish_signal(con: &mut MultiplexedConnection, signal: &Signal) -> Result<()> {
    let payload = serde_json::to_string(signal).context("Signal serialisation")?;
    let _: String = con
        .xadd(STREAM_JAX_SIGNALS, "*", &[("payload", payload)])
        .await
        .context("xadd jax:signals")?;
    Ok(())
}

/// Create the consumer group if it does not yet exist.
async fn ensure_consumer_group(con: &mut MultiplexedConnection) {
    let result: redis::RedisResult<()> = con
        .xgroup_create_mkstream(STREAM_CLEAN_TICKS, GROUP_NAME, "$")
        .await;
    match result {
        Ok(_)  => info!("Consumer group '{GROUP_NAME}' created"),
        Err(e) if e.to_string().contains("BUSYGROUP") => {
            debug!("Consumer group '{GROUP_NAME}' already exists – OK");
        }
        Err(e) => warn!("xgroup_create_mkstream: {e}"),
    }
}

/// XREADGROUP – blocks for up to 2 seconds, returns `(msg_id, Bar)` pairs.
async fn read_messages(con: &mut MultiplexedConnection) -> Result<Vec<(String, Bar)>> {
    let opts = StreamReadOptions::default()
        .group(GROUP_NAME, CONSUMER_NAME)
        .count(BATCH_SIZE)
        .block(2_000); // ms

    let reply: Option<StreamReadReply> = con
        .xread_options(&[STREAM_CLEAN_TICKS], &[">"], &opts)
        .await
        .context("xreadgroup clean:ticks")?;

    let mut out = Vec::new();

    if let Some(reply) = reply {
        for stream in reply.keys {
            for entry in stream.ids {
                if let Some(payload_val) = entry.map.get("payload") {
                    let payload_str = match payload_val {
                        redis::Value::Data(b) => String::from_utf8_lossy(b).into_owned(),
                        redis::Value::Status(s) => s.clone(),
                        _ => continue,
                    };
                    match serde_json::from_str::<Bar>(&payload_str) {
                        Ok(bar) => out.push((entry.id.clone(), bar)),
                        Err(e)  => warn!("Bar deserialise error: {e} | raw={payload_str}"),
                    }
                }
            }
        }
    }

    Ok(out)
}

/// XACK – acknowledge a processed message so it leaves the PEL.
async fn ack_message(con: &mut MultiplexedConnection, msg_id: &str) -> Result<()> {
    let _: i64 = con
        .xack(STREAM_CLEAN_TICKS, GROUP_NAME, &[msg_id])
        .await
        .context("xack clean:ticks")?;
    Ok(())
}

/// Current time as a Unix float (seconds.subseconds).
fn unix_now() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}
