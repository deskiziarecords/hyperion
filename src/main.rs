use hyperion_adco::types::*;
use hyperion_adco::continuum::HamiltonianEngine;
use hyperion_adco::routing::SchurRouter;
use std::time::Instant;

fn main() {
    let mut h_engine = HamiltonianEngine::new();
    let ctx = MarketContext { eq_50: 1.1550, erl_target: 1.1419, irl_fvg: 1.1510 };
    
    // Example: Simulation of an incoming 1-minute Adelic Burst
    let frame = PriceFrame { open: 1.1550, high: 1.1590, low: 1.1545, close: 1.1585, atr: 0.0020 };

    let start = Instant::now();

    // 1. Evaluate Hamiltonian Energy (Manual increment for demo)
    h_engine.energy = 0.92; 

    // 2. λ6 Veto & State Transition
    let veto = h_engine.compute_displacement_veto(&frame);
    h_engine.update_state(frame.close, &ctx, veto);

    // 3. Execution Gate
    if h_engine.state == ContinuumState::Expansion {
        let (dark, lit) = SchurRouter::route(5_000_000.0);
        println!("LATENCY: {:?} | DISPATCHED: Dark={}, Lit={}", start.elapsed(), dark, lit);
    } else {
        println!("GATE CLOSED: State is {:?}", h_engine.state);
    }
}
