use crate::types::Phase;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalState {
    pub ipda_phase: Phase,
    pub kill_zone_active: bool,
    pub current_drawdown: f64,
    pub mandra_gate_level: i32,
    pub ipda_lookback: usize,
    pub accumulation_start_ts: Option<i64>,
    pub last_fft_spectrum: Vec<f64>,
    pub open_positions: Vec<String>,
}

impl Default for GlobalState {
    fn default() -> Self {
        Self {
            ipda_phase: Phase::Flat,
            kill_zone_active: false,
            current_drawdown: 0.0,
            mandra_gate_level: 0,
            ipda_lookback: 20,
            accumulation_start_ts: None,
            last_fft_spectrum: Vec::new(),
            open_positions: Vec::new(),
        }
    }
}

pub type SharedState = Arc<RwLock<GlobalState>>;
