use chrono::{DateTime, Utc};
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt;

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

#[pymethods]
impl Candle {
    #[getter]
    fn get_time(&self) -> String {
        self.time.to_rfc3339()
    }
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

#[pyclass(eq, eq_int)]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum Phase {
    Accumulation,
    Manipulation,
    Distribution,
    Expansion,
    Retracement,
    Reversal,
    Consolidation,
    Flat,
}

impl fmt::Display for Phase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Phase::Accumulation => "ACCUMULATION",
            Phase::Manipulation => "MANIPULATION",
            Phase::Distribution => "DISTRIBUTION",
            Phase::Expansion => "EXPANSION",
            Phase::Retracement => "RETRACEMENT",
            Phase::Reversal => "REVERSAL",
            Phase::Consolidation => "CONSOLIDATION",
            Phase::Flat => "FLAT",
        };
        write!(f, "{}", s)
    }
}

#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Action {
    Buy,
    Sell,
    Flat,
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Action::Buy => "BUY",
            Action::Sell => "SELL",
            Action::Flat => "FLAT",
        };
        write!(f, "{}", s)
    }
}

#[pyclass]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bar {
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
    pub ts: i64,
}

#[pyclass]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    #[pyo3(get)]
    pub action: Action,
    #[pyo3(get)]
    pub size: f64,
    #[pyo3(get)]
    pub phase: Phase,
    #[pyo3(get)]
    pub kill_zone: bool,
    #[pyo3(get)]
    pub timestamp: f64,
    #[pyo3(get)]
    pub price: Option<f64>,
}

#[pyclass]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub action: Action,
    #[pyo3(get)]
    pub size: f64,
    #[pyo3(get)]
    pub ref_price: f64,
    #[pyo3(get)]
    pub timestamp: f64,
    #[pyo3(get)]
    pub symbol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawTick {
    pub price: f64,
    pub volume: f64,
    pub ts_ms: i64,
}
