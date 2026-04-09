use chrono::{DateTime, Utc};
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

pub trait AdelicComponent {
    fn is_coherent(&self, rho: f64) -> bool;
}

#[derive(Debug, Clone, Copy)]
pub struct PriceFrame {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub atr: f64,
}

#[pyclass]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
    #[pyo3(get)]
    pub time: DateTime<Utc>,
    #[pyo3(get)]
    pub open: f64,
    #[pyo3(get)]
    pub high: f64,
    #[pyo3(get)]
    pub low: f64,
    #[pyo3(get)]
    pub close: f64,
    #[pyo3(get)]
    pub volume: f64,
    #[pyo3(get)]
    pub pattern: Pattern,
}

#[pyclass]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub strength: f64,
}

pub struct MarketContext {
    pub eq_50: f64,
    pub erl_target: f64,
    pub irl_fvg: f64,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ContinuumState {
    Consolidation,
    Expansion,
    Retracement,
    Reversal,
    Stasis,
}
