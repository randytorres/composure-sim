//! Scenario definition and validation.
//!
//! A scenario defines the initial conditions, actions, and parameters
//! for a simulation run. Domain-agnostic: consumers define what the
//! dimensions and actions mean.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::state::{Action, SimState};

/// A simulation scenario: initial state + action sequence + parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    /// Unique identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Initial state.
    pub initial_state: SimState,
    /// Actions to apply at each time step.
    /// If shorter than simulation length, remaining steps use `Action::default()`.
    pub actions: Vec<Action>,
    /// Number of time steps to simulate.
    pub time_steps: usize,
    /// Optional: failure threshold for composure analysis.
    /// If health index drops below this, it's flagged as a break point.
    pub failure_threshold: Option<f64>,
    /// Optional metadata.
    pub metadata: Option<serde_json::Value>,
}

impl Scenario {
    pub fn new(id: impl Into<String>, name: impl Into<String>, initial_state: SimState, time_steps: usize) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            initial_state,
            actions: Vec::new(),
            time_steps,
            failure_threshold: None,
            metadata: None,
        }
    }

    /// Validate the scenario before running.
    pub fn validate(&self) -> Result<(), ScenarioError> {
        if self.id.is_empty() {
            return Err(ScenarioError::EmptyId);
        }
        if self.name.is_empty() {
            return Err(ScenarioError::EmptyName);
        }
        if self.time_steps == 0 {
            return Err(ScenarioError::ZeroTimeSteps);
        }
        if self.initial_state.dimensions() == 0 {
            return Err(ScenarioError::ZeroDimensions);
        }
        if let Some(threshold) = self.failure_threshold {
            if !(0.0..=1.0).contains(&threshold) {
                return Err(ScenarioError::InvalidThreshold(threshold));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum ScenarioError {
    #[error("scenario ID cannot be empty")]
    EmptyId,
    #[error("scenario name cannot be empty")]
    EmptyName,
    #[error("time_steps must be > 0")]
    ZeroTimeSteps,
    #[error("initial state must have at least 1 dimension")]
    ZeroDimensions,
    #[error("failure threshold must be in [0, 1], got {0}")]
    InvalidThreshold(f64),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_scenario() {
        let s = Scenario::new("test", "Test Scenario", SimState::zeros(3), 100);
        assert!(s.validate().is_ok());
    }

    #[test]
    fn test_empty_id() {
        let s = Scenario::new("", "Test", SimState::zeros(3), 100);
        assert!(matches!(s.validate(), Err(ScenarioError::EmptyId)));
    }

    #[test]
    fn test_zero_steps() {
        let s = Scenario::new("test", "Test", SimState::zeros(3), 0);
        assert!(matches!(s.validate(), Err(ScenarioError::ZeroTimeSteps)));
    }
}
