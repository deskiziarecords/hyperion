use crate::types::{Bar, Phase};
use crate::indicators::atr;

pub const MAX_BUFFER_BARS: usize = 1440 * 60; // 60 days of 1m bars

/// Detects the IPDA Phase using Adelic heuristics.
/// Heuristics:
/// - Accumulation: Narrow ranges, contracting volume, and price hovering near Equilibrium (EQ).
/// - Manipulation: Sharp, high-volume move counter to the expected direction, often breaching local liquidity.
/// - Distribution: Wide ranges, high volume, and price moving away from EQ towards External Range Liquidity (ERL).
/// - Expansion: Consistent institutional displacement (high body-to-range ratio).
pub fn detect_phase(bars: &[Bar]) -> Phase {
    if bars.len() < 40 {
        return Phase::Flat;
    }

    let last_idx = bars.len() - 1;
    let last = &bars[last_idx];
    let prev = &bars[last_idx - 1];

    let range = last.high - last.low;
    let body = (last.close - last.open).abs();
    let volume = last.volume;

    let avg_volume: f64 = bars.iter().rev().take(20).map(|b| b.volume).sum::<f64>() / 20.0;
    let atr20 = atr(bars, 20);

    // Displacement check (λ6 Displacement)
    let displacement = body / range > 0.7 && range > 1.2 * atr20;

    if displacement {
        if range > 2.5 * atr20 && volume > 1.5 * avg_volume {
            return Phase::Manipulation;
        }
        return Phase::Expansion;
    }

    if range < 0.8 * atr20 && volume < avg_volume {
        return Phase::Accumulation;
    }

    if range > 1.5 * atr20 && volume > avg_volume {
        return Phase::Distribution;
    }

    if (last.close - prev.close).abs() < 0.2 * atr20 {
        return Phase::Consolidation;
    }

    Phase::Consolidation
}
