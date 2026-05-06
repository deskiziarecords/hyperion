use crate::state::SharedState;
use anyhow::Result;
use redis::aio::MultiplexedConnection;

/// UROL ingestion — reads tick data from the Redis stream and updates shared state.
pub async fn run_ingestion(_con: MultiplexedConnection, _state: SharedState) -> Result<()> {
    Ok(())
}

/// UROL watchdog — monitors ingestion health and resets stale state.
pub async fn run_watchdog(_con: MultiplexedConnection, _state: SharedState) -> Result<()> {
    Ok(())
}
