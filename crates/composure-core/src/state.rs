use serde::{Deserialize, Serialize};

/// Simulation state at time `t`.
///
/// Domain-agnostic: the meaning of each dimension is defined by the consumer.
/// For health: z might be [hrv, sleep_quality, recovery, body_comp, metabolic, cardio].
/// For biotech: z might be [viability, expression, potency, stability].
/// For marketing: z might be [sentiment, engagement, share_propensity].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimState {
    /// `z_t`: Current functional state across N dimensions.
    /// Values are typically normalized to [0, 1] but the library doesn't enforce this.
    pub z: Vec<f64>,

    /// `m_t`: Accumulated memory / damage / recovery reserve.
    /// Tracks history effects: adaptation, sensitization, fatigue, hysteresis.
    /// Same dimensionality as `z` (one memory value per state dimension).
    pub m: Vec<f64>,

    /// `u_t`: Uncertainty / out-of-distribution indicator per dimension.
    /// Higher values = less confidence in `z_t` predictions.
    /// Used for OOD gating: if uncertainty is high, widen intervals or abstain.
    pub u: Vec<f64>,

    /// Current time step.
    pub t: usize,
}

impl SimState {
    /// Create a new state with `n` dimensions, all initialized to the given values.
    pub fn new(z: Vec<f64>, m: Vec<f64>, u: Vec<f64>) -> Self {
        assert_eq!(z.len(), m.len(), "z and m must have same dimensionality");
        assert_eq!(z.len(), u.len(), "z and u must have same dimensionality");
        Self { z, m, u, t: 0 }
    }

    /// Create a zero-initialized state with `n` dimensions.
    pub fn zeros(n: usize) -> Self {
        Self {
            z: vec![0.0; n],
            m: vec![0.0; n],
            u: vec![0.5; n], // Start with moderate uncertainty
            t: 0,
        }
    }

    /// Number of dimensions.
    pub fn dimensions(&self) -> usize {
        self.z.len()
    }

    /// Compute a single scalar health/performance index from the state.
    /// Default: mean of all z dimensions. Override via `Simulator::health_index()`.
    pub fn default_health_index(&self) -> f64 {
        if self.z.is_empty() {
            return 0.0;
        }
        self.z.iter().sum::<f64>() / self.z.len() as f64
    }
}

/// An action/intervention applied at time `t`.
///
/// Domain-agnostic: `ActionType` variants are extensible.
/// The `Simulator` trait interprets what each action means.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    /// Which dimension this action primarily affects (index into z/m/u).
    /// `None` means it affects the system globally.
    pub dimension: Option<usize>,

    /// Magnitude of the action. Interpretation is domain-specific.
    /// Positive = beneficial intervention, negative = stressor. (Convention, not enforced.)
    pub magnitude: f64,

    /// What kind of action this is.
    pub action_type: ActionType,

    /// Optional metadata (domain-specific).
    pub metadata: Option<serde_json::Value>,
}

/// Action type. Extensible via `Custom(String)`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActionType {
    /// A protocol or intervention change (supplement, treatment, etc.)
    Intervention,
    /// External stressor onset (stress, temperature, travel, etc.)
    StressorOnset,
    /// External stressor removal
    StressorRemoval,
    /// No action (hold current state, let natural dynamics play out)
    Hold,
    /// Domain-specific action type
    Custom(String),
}

impl Default for Action {
    fn default() -> Self {
        Self {
            dimension: None,
            magnitude: 0.0,
            action_type: ActionType::Hold,
            metadata: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_state() {
        let s = SimState::new(vec![0.5, 0.7], vec![0.0, 0.0], vec![0.5, 0.5]);
        assert_eq!(s.dimensions(), 2);
        assert_eq!(s.t, 0);
    }

    #[test]
    fn test_zeros() {
        let s = SimState::zeros(6);
        assert_eq!(s.dimensions(), 6);
        assert!(s.z.iter().all(|&v| v == 0.0));
        assert!(s.u.iter().all(|&v| v == 0.5));
    }

    #[test]
    fn test_health_index() {
        let s = SimState::new(vec![0.8, 0.6, 0.4], vec![0.0; 3], vec![0.5; 3]);
        assert!((s.default_health_index() - 0.6).abs() < 1e-10);
    }

    #[test]
    #[should_panic(expected = "z and m must have same dimensionality")]
    fn test_mismatched_dimensions() {
        SimState::new(vec![0.5], vec![0.0, 0.0], vec![0.5]);
    }
}
