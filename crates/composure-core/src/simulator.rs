use crate::state::{Action, SimState};

/// The core trait that domain-specific code implements.
///
/// The library provides orchestration (Monte Carlo, replay, composure analysis).
/// You provide the transition function (what happens when an action is applied to a state).
///
/// # Example
///
/// ```rust
/// use composure_core::{Simulator, SimState, Action};
/// use rand::Rng;
///
/// struct HealthSimulator;
///
/// impl Simulator for HealthSimulator {
///     fn step(&self, state: &SimState, action: &Action, rng: &mut dyn rand::RngCore) -> SimState {
///         let mut next = state.clone();
///         next.t += 1;
///         // Domain-specific: how health state evolves under this action
///         for i in 0..next.z.len() {
///             let noise = (rng.gen::<f64>() - 0.5) * 0.02;
///             let damage = next.m[i] * 0.01; // Memory causes drag
///             next.z[i] = (next.z[i] + action.magnitude * 0.1 + noise - damage).clamp(0.0, 1.0);
///             // Memory accumulates damage, decays over time
///             next.m[i] = (next.m[i] * 0.95 + (1.0 - next.z[i]) * 0.05).clamp(0.0, 1.0);
///         }
///         next
///     }
///
///     fn health_index(&self, state: &SimState) -> f64 {
///         state.default_health_index()
///     }
/// }
/// ```
pub trait Simulator: Send + Sync {
    /// Transition function: compute the next state given current state + action.
    ///
    /// `rng` is provided for stochastic simulation. Use it for all randomness
    /// to ensure deterministic replay when seeded.
    fn step(&self, state: &SimState, action: &Action, rng: &mut dyn rand::RngCore) -> SimState;

    /// Apply zero or more actions within a single timestep.
    ///
    /// Default behavior applies actions sequentially while preserving the outer
    /// timestep, which lets scenario-level conditional actions coexist without
    /// forcing all simulators to reimplement batching.
    fn step_actions(
        &self,
        state: &SimState,
        actions: &[Action],
        rng: &mut dyn rand::RngCore,
    ) -> SimState {
        if actions.is_empty() {
            return self.step(state, &Action::default(), rng);
        }

        let mut next = state.clone();
        for action in actions {
            next = self.step(&next, action, rng);
            next.t = state.t;
        }
        next.t = state.t + 1;
        next
    }

    /// Compute a scalar health/performance index from state.
    /// Used for composure curve analysis and trajectory projection.
    /// Default: mean of z dimensions.
    fn health_index(&self, state: &SimState) -> f64 {
        state.default_health_index()
    }

    /// Optional: compute per-dimension scores (for richer composure analysis).
    /// Default: returns z directly.
    fn dimension_scores(&self, state: &SimState) -> Vec<f64> {
        state.z.clone()
    }
}
