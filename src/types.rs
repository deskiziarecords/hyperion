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
