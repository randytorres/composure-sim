//! Scenario definition and validation.
//!
//! A scenario defines the initial conditions, scheduled actions, and optional
//! conditional rules for a simulation run. Domain-agnostic: consumers define
//! what the dimensions and actions mean.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::state::{Action, SimState};

/// A simulation scenario: initial state + action sequence + conditional rules.
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
    /// Optional state-dependent rules evaluated during scenario execution.
    #[serde(default)]
    pub conditional_actions: Vec<ConditionalActionRule>,
    /// Optional: failure threshold for composure analysis.
    /// If health index drops below this, it's flagged as a break point.
    pub failure_threshold: Option<f64>,
    /// Optional metadata.
    pub metadata: Option<serde_json::Value>,
}

impl Scenario {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        initial_state: SimState,
        time_steps: usize,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            initial_state,
            actions: Vec::new(),
            time_steps,
            conditional_actions: Vec::new(),
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

        let dims = self.initial_state.dimensions();

        if let Some((action_index, dimension)) = self
            .actions
            .iter()
            .enumerate()
            .find_map(|(idx, action)| action.dimension.filter(|&d| d >= dims).map(|d| (idx, d)))
        {
            return Err(ScenarioError::InvalidActionDimension {
                action_index,
                dimension,
                dimensions: dims,
            });
        }

        if let Some(err) = self
            .conditional_actions
            .iter()
            .enumerate()
            .find_map(|(rule_index, rule)| rule.validate(rule_index, dims).err())
        {
            return Err(err);
        }

        let mut conditional_ids = BTreeSet::new();
        if let Some(duplicate_id) = self
            .conditional_actions
            .iter()
            .find_map(|rule| (!conditional_ids.insert(rule.id.clone())).then(|| rule.id.clone()))
        {
            return Err(ScenarioError::DuplicateConditionalActionId(duplicate_id));
        }

        if let Some(threshold) = self.failure_threshold {
            if !(0.0..=1.0).contains(&threshold) {
                return Err(ScenarioError::InvalidThreshold(threshold));
            }
        }

        Ok(())
    }
}

/// A state-dependent rule that can schedule an action during execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionalActionRule {
    /// Stable identifier for deterministic debugging and future reporting.
    pub id: String,
    /// Predicate evaluated after each step against the resulting state.
    /// Crossing variants compare the previous state to the resulting state.
    pub trigger: ConditionalTrigger,
    /// Action to schedule when the trigger matches.
    pub action: Action,
    /// Delay before the action is applied. `0` means the next step.
    #[serde(default)]
    pub delay_steps: usize,
    /// Minimum number of steps before the same rule can fire again.
    #[serde(default)]
    pub cooldown_steps: usize,
    /// Rules with higher priority are applied earlier within the same step.
    #[serde(default)]
    pub priority: i32,
    /// Optional cap on the number of times the rule may fire.
    #[serde(alias = "max_triggers")]
    pub max_fires: Option<usize>,
}

impl ConditionalActionRule {
    fn validate(&self, rule_index: usize, dimensions: usize) -> Result<(), ScenarioError> {
        if self.id.trim().is_empty() {
            return Err(ScenarioError::EmptyConditionalActionId { rule_index });
        }

        self.trigger.validate(rule_index, dimensions)?;

        if let Some(dimension) = self.action.dimension {
            if dimension >= dimensions {
                return Err(ScenarioError::InvalidConditionalActionDimension {
                    rule_index,
                    dimension,
                    dimensions,
                });
            }
        }

        if !self.action.magnitude.is_finite() {
            return Err(ScenarioError::NonFiniteConditionalActionMagnitude { rule_index });
        }

        if let Some(max_fires) = self.max_fires {
            if max_fires == 0 {
                return Err(ScenarioError::InvalidConditionalActionMaxFires { rule_index });
            }
        }

        Ok(())
    }
}

/// Threshold-based predicates supported by the first conditional action system.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ConditionalTrigger {
    HealthIndexBelow { threshold: f64 },
    HealthIndexAbove { threshold: f64 },
    HealthIndexCrossesBelow { threshold: f64 },
    HealthIndexCrossesAbove { threshold: f64 },
    DimensionBelow { dimension: usize, threshold: f64 },
    DimensionAbove { dimension: usize, threshold: f64 },
    DimensionCrossesBelow { dimension: usize, threshold: f64 },
    DimensionCrossesAbove { dimension: usize, threshold: f64 },
}

impl ConditionalTrigger {
    fn validate(&self, rule_index: usize, dimensions: usize) -> Result<(), ScenarioError> {
        match self {
            Self::HealthIndexBelow { threshold }
            | Self::HealthIndexAbove { threshold }
            | Self::HealthIndexCrossesBelow { threshold }
            | Self::HealthIndexCrossesAbove { threshold } => {
                validate_conditional_threshold(rule_index, *threshold)?;
            }
            Self::DimensionBelow {
                dimension,
                threshold,
            }
            | Self::DimensionAbove {
                dimension,
                threshold,
            }
            | Self::DimensionCrossesBelow {
                dimension,
                threshold,
            }
            | Self::DimensionCrossesAbove {
                dimension,
                threshold,
            } => {
                if *dimension >= dimensions {
                    return Err(ScenarioError::InvalidConditionalTriggerDimension {
                        rule_index,
                        dimension: *dimension,
                        dimensions,
                    });
                }
                validate_conditional_threshold(rule_index, *threshold)?;
            }
        }

        Ok(())
    }
}

fn validate_conditional_threshold(rule_index: usize, threshold: f64) -> Result<(), ScenarioError> {
    if !threshold.is_finite() {
        return Err(ScenarioError::NonFiniteConditionalThreshold { rule_index });
    }
    if !(0.0..=1.0).contains(&threshold) {
        return Err(ScenarioError::InvalidConditionalThreshold {
            rule_index,
            threshold,
        });
    }
    Ok(())
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
    #[error("action {action_index} targets dimension {dimension}, but state has {dimensions} dimensions")]
    InvalidActionDimension {
        action_index: usize,
        dimension: usize,
        dimensions: usize,
    },
    #[error("conditional action rule {rule_index} ID cannot be empty")]
    EmptyConditionalActionId { rule_index: usize },
    #[error("conditional action rule ID {0} must be unique")]
    DuplicateConditionalActionId(String),
    #[error(
        "conditional action rule {rule_index} trigger targets dimension {dimension}, but state has {dimensions} dimensions"
    )]
    InvalidConditionalTriggerDimension {
        rule_index: usize,
        dimension: usize,
        dimensions: usize,
    },
    #[error(
        "conditional action rule {rule_index} action targets dimension {dimension}, but state has {dimensions} dimensions"
    )]
    InvalidConditionalActionDimension {
        rule_index: usize,
        dimension: usize,
        dimensions: usize,
    },
    #[error("conditional action rule {rule_index} threshold must be finite")]
    NonFiniteConditionalThreshold { rule_index: usize },
    #[error("conditional action rule {rule_index} threshold must be in [0, 1], got {threshold}")]
    InvalidConditionalThreshold { rule_index: usize, threshold: f64 },
    #[error("conditional action rule {rule_index} magnitude must be finite")]
    NonFiniteConditionalActionMagnitude { rule_index: usize },
    #[error("conditional action rule {rule_index} must set max_fires > 0 when provided")]
    InvalidConditionalActionMaxFires { rule_index: usize },
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

    #[test]
    fn test_invalid_action_dimension() {
        let mut s = Scenario::new("test", "Test", SimState::zeros(3), 10);
        s.actions.push(Action {
            dimension: Some(3),
            magnitude: 1.0,
            action_type: crate::state::ActionType::Intervention,
            metadata: None,
        });

        assert!(matches!(
            s.validate(),
            Err(ScenarioError::InvalidActionDimension {
                action_index: 0,
                dimension: 3,
                dimensions: 3,
            })
        ));
    }

    #[test]
    fn test_invalid_conditional_trigger_dimension() {
        let mut s = Scenario::new("test", "Test", SimState::zeros(2), 10);
        s.conditional_actions.push(ConditionalActionRule {
            id: "rule-1".into(),
            trigger: ConditionalTrigger::DimensionBelow {
                dimension: 2,
                threshold: 0.3,
            },
            action: Action {
                dimension: Some(0),
                magnitude: 0.2,
                action_type: crate::state::ActionType::Intervention,
                metadata: None,
            },
            delay_steps: 0,
            cooldown_steps: 0,
            priority: 1,
            max_fires: None,
        });

        assert!(matches!(
            s.validate(),
            Err(ScenarioError::InvalidConditionalTriggerDimension {
                rule_index: 0,
                dimension: 2,
                dimensions: 2,
            })
        ));
    }

    #[test]
    fn test_invalid_conditional_action_dimension() {
        let mut s = Scenario::new("test", "Test", SimState::zeros(2), 10);
        s.conditional_actions.push(ConditionalActionRule {
            id: "rule-1".into(),
            trigger: ConditionalTrigger::HealthIndexBelow { threshold: 0.3 },
            action: Action {
                dimension: Some(2),
                magnitude: 0.2,
                action_type: crate::state::ActionType::Intervention,
                metadata: None,
            },
            delay_steps: 0,
            cooldown_steps: 0,
            priority: 1,
            max_fires: None,
        });

        assert!(matches!(
            s.validate(),
            Err(ScenarioError::InvalidConditionalActionDimension {
                rule_index: 0,
                dimension: 2,
                dimensions: 2,
            })
        ));
    }

    #[test]
    fn test_duplicate_conditional_action_ids_rejected() {
        let mut s = Scenario::new("test", "Test", SimState::zeros(1), 10);
        for _ in 0..2 {
            s.conditional_actions.push(ConditionalActionRule {
                id: "rule-1".into(),
                trigger: ConditionalTrigger::HealthIndexBelow { threshold: 0.3 },
                action: Action::default(),
                delay_steps: 0,
                cooldown_steps: 0,
                priority: 1,
                max_fires: None,
            });
        }

        assert!(matches!(
            s.validate(),
            Err(ScenarioError::DuplicateConditionalActionId(id)) if id == "rule-1"
        ));
    }

    #[test]
    fn test_invalid_conditional_threshold_rejected() {
        let mut s = Scenario::new("test", "Test", SimState::zeros(1), 10);
        s.conditional_actions.push(ConditionalActionRule {
            id: "rule-1".into(),
            trigger: ConditionalTrigger::HealthIndexBelow { threshold: 1.5 },
            action: Action::default(),
            delay_steps: 0,
            cooldown_steps: 0,
            priority: 1,
            max_fires: None,
        });

        assert!(matches!(
            s.validate(),
            Err(ScenarioError::InvalidConditionalThreshold {
                rule_index: 0,
                threshold
            }) if (threshold - 1.5).abs() < f64::EPSILON
        ));
    }

    #[test]
    fn test_invalid_max_fires_rejected() {
        let mut s = Scenario::new("test", "Test", SimState::zeros(1), 10);
        s.conditional_actions.push(ConditionalActionRule {
            id: "rule-1".into(),
            trigger: ConditionalTrigger::HealthIndexBelow { threshold: 0.3 },
            action: Action::default(),
            delay_steps: 0,
            cooldown_steps: 0,
            priority: 1,
            max_fires: Some(0),
        });

        assert!(matches!(
            s.validate(),
            Err(ScenarioError::InvalidConditionalActionMaxFires { rule_index: 0 })
        ));
    }
}
