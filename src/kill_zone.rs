use crate::state::SharedState;
use anyhow::Result;
use redis::aio::MultiplexedConnection;

/// Kill zone detection — monitors session timing and updates shared state.
pub async fn run(_con: MultiplexedConnection, state: SharedState) -> Result<()> {
    let mut s = state.write().await;
    s.kill_zone_active = false;
    Ok(())
}
