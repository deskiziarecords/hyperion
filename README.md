# Hyperion IPDA Strategy Tool

Hyperion is a high-performance Rust-based implementation of an **Interbank Price Delivery Algorithm (IPDA)** strategy tool. It leverages physical and mathematical analogies (Hamiltonian mechanics, Schur decomposition, and Adelic theory) to model market dynamics and execute institutional-grade order routing.

## Core Architecture

### 1. Hamiltonian Engine (`src/continuum.rs`)
The `HamiltonianEngine` serves as the primary state machine for the strategy. It tracks the "energy" of the market to determine phase transitions.
- **State Machine**: Transitions through `Consolidation`, `Expansion`, `Retracement`, `Reversal`, and `Stasis`.
- **Displacement Veto**: A λ6-inspired filter that identifies high-conviction institutional moves by evaluating price range and body ratios relative to ATR (Average True Range).
- **State Transitions**:
  - `Consolidation` -> `Expansion`: Triggered when energy exceeds 0.85 and no displacement veto is active.
  - `Expansion` -> `Reversal`: Occurs when price approaches the ERL (External Range Liquidity) target.

### 2. Schur Router (`src/routing.rs`)
The `SchurRouter` handles the dispatching of large volume orders across multiple venues.
- **Golden Ratio Routing**: Dispatches approximately 61.8% of volume (the Golden Ratio, or Φ) to Dark Pools for primary block execution, with the remainder routed to Lit venues to maintain liquidity profile coherence.

### 3. Adelic Types (`src/types.rs`)
Core data structures that define the market environment.
- **AdelicComponent**: A trait for ensuring coherence in price delivery across different scales.
- **PriceFrame**: Encapsulates standard OHLC data along with ATR for volatility analysis.
- **MarketContext**: Tracks key levels including EQ (Equilibrium) 50%, ERL (External Range Liquidity) targets, and IRL (Internal Range Liquidity) FVG (Fair Value Gaps).

## Getting Started

### Prerequisites
- Rust (Edition 2021)
- Cargo

### Building the Project
```bash
cargo build
```

### Running the Simulation
The simulation demonstrates an incoming "Adelic Burst" and how the engine processes it:
```bash
cargo run
```

### Running Tests
```bash
cargo test
```

## Mathematical Philosophy
Hyperion's design philosophy is rooted in the idea that market price delivery follows a deterministic, non-linear continuum. By using Hamiltonian energy thresholds, it filters out noise and focuses on institutional-scale liquidity cycles.
