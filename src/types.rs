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

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
#[pyclass]
pub enum Phase {
    Accumulation,
    Manipulation,
    Distribution,
    Expansion,
    Retracement,
    Reversal,
    Flat,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[pyclass]
pub enum Action {
    Buy,
    Sell,
    Flat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[pyclass]
pub struct Bar {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub ts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[pyclass]
pub struct Signal {
    pub action: Action,
    pub size: f64,
    pub phase: Phase,
    pub kill_zone: bool,
    pub timestamp: f64,
    pub price: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[pyclass]
pub struct Order {
    pub id: String,
    pub action: Action,
    pub size: f64,
    pub ref_price: f64,
    pub timestamp: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawTick {
    pub price: f64,
    pub volume: f64,
    pub ts_ms: i64,
}
