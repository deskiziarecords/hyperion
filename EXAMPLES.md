# Hyperion-7ZERO: Mission Parameters & Use Cases

This playbook defines how the unified Sentinel (Rust) and Adelic Brain (JAX) system behaves under specific market conditions. These scenarios serve as the "Mission Parameters" for the institutional-grade execution demo.

---

### 1. The "Impulsive Breakout" (λ6 Veto in Action)
**Scenario:** A sudden news spike causes a 30-pip jump in EUR/USD within a single M1 candle.

**System Response:**
*   **Hyperion (Rust):** Detects the tick surge. `CandleBuilder` identifies a "U" (Upward) or "B" (Expansion) pattern.
*   **7ZERO (JAX):** Runs the λ6 check. Calculates a body-to-range ratio of 0.95.
*   **The Veto:** The orchestrator checks if `body / range > 0.75`. Because institutional moves have large bodies, if the ratio is too low (e.g. 0.40), it determines this is an "Impulsive Wick" (likely a stop-run) and vetoes the trade.
*   **Outcome:** The system stays flat, avoiding a potential "stop-run" reversal that would have trapped a less sophisticated bot.

---

### 2. The "Invisible Accumulation" (Schur Routing)
**Scenario:** The JAX Brain identifies a high-stability Adelic Manifold (Stability > 0.90) and signals a BUY for 5,000,000 units (50 lots).

**System Response:**
*   **Orchestrator:** Passes the 5M unit signal back to the Rust `execute_trade` bridge.
*   **Schur Router (Rust):** Applies the **Golden Ratio (Φ)** to minimize market footprint.
*   **Execution:**
    *   **3,090,000 units (61.8%)** are routed to **Dark Pools** (hidden execution).
    *   **1,910,000 units (38.2%)** are routed to **Lit Venues** (public exchanges).
*   **Outcome:** The total position is filled with minimal price impact (slippage < 0.2 pips), maintaining the "Invisible Trader" protocol.

---

### 3. The "Regime Fracture" (Circuit Breaker)
**Scenario:** Market volatility shifts from trending to chaotic "random walk" noise, causing the mathematical model to lose coherence.

**System Response:**
*   **Adelic-Koopman Sync:** The spectral radius of the market state exceeds the coherence limit.
*   **Brain:** Stability drops below the **0.85 threshold**.
*   **Kill Switch:** The orchestrator detects `stability < 0.85`, triggers `self.engine.cancel_all_pending()`, and pauses execution.
*   **Outcome:** All pending orders are cancelled, and the system enters a "Cool-down" mode to preserve capital until the market regime stabilizes.
