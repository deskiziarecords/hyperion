pub const STREAM_JAX_SIGNALS: &str = "jax:signals";
pub const STREAM_CLEAN_TICKS: &str = "clean:ticks";
pub const STATE_KEY: &str = "quimeria:state";
pub const ATR_PERIOD: usize = 20;
pub const BUCKET_MS: i64 = 60_000;
pub const EQUITY: f64 = 100_000.0;
pub const RISK_PER_TRADE: f64 = 0.01;
pub const PIP_VALUE: f64 = 10.0;
pub const BARS_PER_DAY: usize = 1440;
pub const LOOKBACK_DAYS: [usize; 3] = [20, 40, 60];
pub const KILL_ZONES: [(u32, u32); 2] = [
    (8, 12),  // London
    (12, 18), // New York
];
