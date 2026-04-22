import jax
import jax.numpy as jnp
import numpy as np
from dataclasses import dataclass

@dataclass
class MarketState:
    signal_history: jnp.ndarray  # Ŝ_t history
    price_history: jnp.ndarray   # P_t history
    lambda_vector: jnp.ndarray   # Current firing state of λ1-λ5
    prev_posterior: float        # P(Z_{t-1}=1)

class EdgeFlipDetector:
    def __init__(self, alpha_persist=0.92, threshold=0.65):
        self.alpha = alpha_persist
        self.tau = threshold
        # Dirichlet priors for sensor reliability θ_i = P(λ_i=1 | Z=1)
        self.alphas = jnp.ones(5) * 10.0
        self.betas = jnp.ones(5) * 2.0 

    @jax.jit
    def calculate_expectancy_inversion(self, signals, returns):
        """
        Detects the 'Sign Flip' in expected returns.
        Returns negative if the strategy edge has inverted.
        """
        # Covariance(Ŝ_t, ΔP_{t+1})
        covariance = jnp.mean(signals * returns) - jnp.mean(signals) * jnp.mean(returns)
        return covariance < 0

    @jax.jit
    def obnfe_posterior(self, state: MarketState):
        """
        Online Bayesian Network Fusion Engine (OBNFE) Core Equation.
        Calculates P(Z_t=1 | Λ_t) using Hidden Markov Prior.
        """
        # 1. Hidden Markov Prior (Regime Persistence)
        pi_t = self.alpha * state.prev_posterior + (1 - self.alpha) * (1 - state.prev_posterior)
        
        # 2. Likelihood calculation (Noisy Sensors λ1-λ5)
        theta = self.alphas / (self.alphas + self.betas) # Reliability
        phi = 1.0 - theta # Noise
        
        likelihood_z1 = jnp.prod(jnp.power(theta, state.lambda_vector) * 
                                 jnp.power(1-theta, 1-state.lambda_vector))
        likelihood_z0 = jnp.prod(jnp.power(phi, state.lambda_vector) * 
                                 jnp.power(1-phi, 1-state.lambda_vector))
        
        # 3. Bayes Theorem Update
        numerator = pi_t * likelihood_z1
        denominator = numerator + (1 - pi_t) * likelihood_z0
        
        posterior = numerator / (denominator + 1e-9)
        return posterior

    def monitor_edge(self, state: MarketState):
        """
        Main execution gate. Triggers mandatory trade halt (u_t=0) on edge flip.
        """
        # Calculate inversion and Bayesian belief
        returns = jnp.diff(jnp.log(state.price_history))
        signals = state.signal_history[:-1]
        
        edge_is_inverted = self.calculate_expectancy_inversion(signals, returns)
        p_failure = self.obnfe_posterior(state)
        
        # Decision Rule: R_t = 1[P(Z_t=1 | Λ_t) > τ]
        halt_trading = p_failure > self.tau
        
        status = "REVERSE_PERIOD_DETECTED" if halt_trading else "NORMAL_REGIME"
        
        return {
            "status": status,
            "p_reverse": float(p_failure),
            "expectancy_inverted": bool(edge_is_inverted),
            "action": "HALT_EXECUTION" if halt_trading else "PROCEED"
        }

# --- Example Usage ---
# Simulation of a failure cascade (λ3 Spectral Inversion firing)
current_state = MarketState(
    signal_history=jnp.array([0.8, 0.85, 0.9, 0.95]), # High confidence (Paradox)
    price_history=jnp.array([1.1500, 1.1510, 1.1505, 1.1490]), # Falling price
    lambda_vector=jnp.array([5]), # λ3 and λ4 firing
    prev_posterior=0.12
)

detector = EdgeFlipDetector()
result = detector.monitor_edge(current_state)
print(f"System State: {result['status']} | P(Reverse): {result['p_reverse']:.2%}")
# Output should indicate if the Bayesian Kill Switch overrides the high-confidence signal.

