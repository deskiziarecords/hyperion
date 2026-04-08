use crate::types::*;
use crate::continuum::HamiltonianEngine;
use crate::routing::SchurRouter;
use crate::engine::state::MarketState;

pub struct ExecutionEngine {
    pub h_engine: HamiltonianEngine,
    pub state: MarketState,
}

impl ExecutionEngine {
    pub fn new(window_size: usize) -> Self {
        Self {
            h_engine: HamiltonianEngine::new(),
            state: MarketState::new(window_size),
        }
    }

    pub fn process_candle(&mut self, candle: Candle) {
        // Update state/memory
        self.state.update(candle.clone());

        // Use the current context (simulated for now)
        let ctx = MarketContext {
            eq_50: 1.1550,
            erl_target: 1.1419,
            irl_fvg: 1.1510
        };

        // Map Candle to PriceFrame for the HamiltonianEngine
        let frame = PriceFrame {
            open: candle.open,
            high: candle.high,
            low: candle.low,
            close: candle.close,
            atr: 0.0020, // Simplified: should ideally be calculated from state
        };

        // Hamiltonian Logic
        // In a real scenario, energy might be derived from multiple candles
        self.h_engine.energy = 0.92;
        let veto = self.h_engine.compute_displacement_veto(&frame);
        self.h_engine.update_state(frame.close, &ctx, veto);

        // Execution Gate
        if self.h_engine.state == ContinuumState::Expansion {
            let (dark, lit) = SchurRouter::route(5_000_000.0);
            println!(
                "SIGNAL: EXPANSION | DISPATCHED: Dark={}, Lit={} | Pattern: {:?}",
                dark, lit, candle.pattern
            );
        }
    }
}
