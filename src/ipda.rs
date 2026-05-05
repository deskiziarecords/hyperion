use crate::state::SharedState;
use crate::types::Phase;
use anyhow::Result;
use redis::aio::MultiplexedConnection;

/// IPDA core — classifies AMD cycle phase and publishes to shared state.
pub async fn run(_con: MultiplexedConnection, state: SharedState) -> Result<()> {
    let mut s = state.write().await;
    if s.ipda_phase == Phase::Flat {
        s.ipda_phase = Phase::Accumulation;
    }
    Ok(())
}
