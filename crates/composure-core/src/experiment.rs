use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    ComposureCurve, MonteCarloConfig, MonteCarloResult, ReplayRun, Scenario, ScenarioError,
};

/// Portable description of an experiment and its default execution settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentSpec {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub scenario: Scenario,
    pub default_monte_carlo: Option<MonteCarloConfig>,
    pub tags: Vec<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at_unix_s: u64,
}

impl ExperimentSpec {
    pub fn new(id: impl Into<String>, name: impl Into<String>, scenario: Scenario) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            scenario,
            default_monte_carlo: None,
            tags: Vec::new(),
            metadata: None,
            created_at_unix_s: now_unix_s(),
        }
    }

    pub fn validate(&self) -> Result<(), ExperimentError> {
        validate_id(&self.id, "experiment spec ID")?;
        validate_name(&self.name, "experiment spec name")?;
        self.scenario
            .validate()
            .map_err(|err| ExperimentError::InvalidScenario {
                context: "experiment spec",
                source: err,
            })?;
        if let Some(config) = &self.default_monte_carlo {
            config
                .validate()
                .map_err(ExperimentError::InvalidMonteCarloConfig)?;
        }
        Ok(())
    }
}

/// Named variant or override set inside an experiment bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentParameterSet {
    pub id: String,
    pub name: String,
    pub scenario: Scenario,
    pub monte_carlo: Option<MonteCarloConfig>,
    pub notes: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

impl ExperimentParameterSet {
    pub fn new(id: impl Into<String>, name: impl Into<String>, scenario: Scenario) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            scenario,
            monte_carlo: None,
            notes: None,
            metadata: None,
        }
    }

    pub fn validate(&self) -> Result<(), ExperimentError> {
        validate_id(&self.id, "parameter set ID")?;
        validate_name(&self.name, "parameter set name")?;
        self.scenario
            .validate()
            .map_err(|err| ExperimentError::InvalidScenario {
                context: "parameter set",
                source: err,
            })?;
        if let Some(config) = &self.monte_carlo {
            config
                .validate()
                .map_err(ExperimentError::InvalidMonteCarloConfig)?;
        }
        Ok(())
    }
}

/// Collected outputs from a simulation run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentOutcome {
    pub monte_carlo: Option<MonteCarloResult>,
    pub composure: Option<ComposureCurve>,
    pub replay: Option<ReplayRun>,
    pub metadata: Option<serde_json::Value>,
}

impl ExperimentOutcome {
    pub fn from_monte_carlo(result: MonteCarloResult) -> Self {
        Self {
            monte_carlo: Some(result),
            composure: None,
            replay: None,
            metadata: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExperimentRunStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

/// Persistent record for a single run of an experiment variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentRunRecord {
    pub run_id: String,
    pub parameter_set_id: Option<String>,
    pub status: ExperimentRunStatus,
    pub seed: Option<u64>,
    pub started_at_unix_s: u64,
    pub completed_at_unix_s: Option<u64>,
    pub outcome: Option<ExperimentOutcome>,
    pub error: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

impl ExperimentRunRecord {
    pub fn running(
        run_id: impl Into<String>,
        parameter_set_id: Option<impl Into<String>>,
        seed: Option<u64>,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            parameter_set_id: parameter_set_id.map(|value| value.into()),
            status: ExperimentRunStatus::Running,
            seed,
            started_at_unix_s: now_unix_s(),
            completed_at_unix_s: None,
            outcome: None,
            error: None,
            metadata: None,
        }
    }

    pub fn mark_completed(mut self, outcome: ExperimentOutcome) -> Self {
        self.status = ExperimentRunStatus::Completed;
        self.completed_at_unix_s = Some(now_unix_s());
        self.outcome = Some(outcome);
        self.error = None;
        self
    }

    pub fn mark_failed(mut self, error: impl Into<String>) -> Self {
        self.status = ExperimentRunStatus::Failed;
        self.completed_at_unix_s = Some(now_unix_s());
        self.error = Some(error.into());
        self.outcome = None;
        self
    }

    pub fn validate(&self) -> Result<(), ExperimentError> {
        validate_id(&self.run_id, "run ID")?;
        match self.status {
            ExperimentRunStatus::Completed if self.outcome.is_none() => {
                return Err(ExperimentError::CompletedRunMissingOutcome(
                    self.run_id.clone(),
                ))
            }
            ExperimentRunStatus::Failed if self.error.as_deref().unwrap_or("").is_empty() => {
                return Err(ExperimentError::FailedRunMissingError(self.run_id.clone()))
            }
            _ => {}
        }
        Ok(())
    }
}

/// Serializable package that downstream tools can save, diff, and rerun.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentBundle {
    pub spec: ExperimentSpec,
    pub parameter_sets: Vec<ExperimentParameterSet>,
    pub runs: Vec<ExperimentRunRecord>,
}

impl ExperimentBundle {
    pub fn new(spec: ExperimentSpec) -> Self {
        Self {
            spec,
            parameter_sets: Vec::new(),
            runs: Vec::new(),
        }
    }

    pub fn add_parameter_set(
        &mut self,
        parameter_set: ExperimentParameterSet,
    ) -> Result<(), ExperimentError> {
        parameter_set.validate()?;
        if self
            .parameter_sets
            .iter()
            .any(|existing| existing.id == parameter_set.id)
        {
            return Err(ExperimentError::DuplicateParameterSet(parameter_set.id));
        }
        self.parameter_sets.push(parameter_set);
        Ok(())
    }

    pub fn record_run(&mut self, run: ExperimentRunRecord) -> Result<(), ExperimentError> {
        run.validate()?;
        if self
            .runs
            .iter()
            .any(|existing| existing.run_id == run.run_id)
        {
            return Err(ExperimentError::DuplicateRunId(run.run_id));
        }
        if let Some(parameter_set_id) = &run.parameter_set_id {
            if !self
                .parameter_sets
                .iter()
                .any(|set| &set.id == parameter_set_id)
            {
                return Err(ExperimentError::UnknownParameterSet(
                    parameter_set_id.clone(),
                ));
            }
        }
        self.runs.push(run);
        Ok(())
    }

    pub fn validate(&self) -> Result<(), ExperimentError> {
        self.spec.validate()?;

        for parameter_set in &self.parameter_sets {
            parameter_set.validate()?;
        }
        for run in &self.runs {
            run.validate()?;
            if let Some(parameter_set_id) = &run.parameter_set_id {
                if !self
                    .parameter_sets
                    .iter()
                    .any(|set| &set.id == parameter_set_id)
                {
                    return Err(ExperimentError::UnknownParameterSet(
                        parameter_set_id.clone(),
                    ));
                }
            }
        }

        ensure_unique_ids(
            self.parameter_sets.iter().map(|set| set.id.as_str()),
            ExperimentError::DuplicateParameterSet,
        )?;
        ensure_unique_ids(
            self.runs.iter().map(|run| run.run_id.as_str()),
            ExperimentError::DuplicateRunId,
        )?;

        Ok(())
    }
}

fn validate_id(id: &str, field: &'static str) -> Result<(), ExperimentError> {
    if id.trim().is_empty() {
        return Err(ExperimentError::EmptyField(field));
    }
    Ok(())
}

fn validate_name(name: &str, field: &'static str) -> Result<(), ExperimentError> {
    if name.trim().is_empty() {
        return Err(ExperimentError::EmptyField(field));
    }
    Ok(())
}

fn ensure_unique_ids<'a, I, F>(ids: I, error: F) -> Result<(), ExperimentError>
where
    I: Iterator<Item = &'a str>,
    F: Fn(String) -> ExperimentError,
{
    let mut seen = std::collections::BTreeSet::new();
    for id in ids {
        if !seen.insert(id) {
            return Err(error(id.to_string()));
        }
    }
    Ok(())
}

fn now_unix_s() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_secs()
}

#[derive(Debug, Error)]
pub enum ExperimentError {
    #[error("{0} cannot be empty")]
    EmptyField(&'static str),
    #[error("invalid scenario in {context}: {source}")]
    InvalidScenario {
        context: &'static str,
        #[source]
        source: ScenarioError,
    },
    #[error("invalid Monte Carlo configuration: {0}")]
    InvalidMonteCarloConfig(crate::MonteCarloError),
    #[error("duplicate parameter set ID: {0}")]
    DuplicateParameterSet(String),
    #[error("duplicate run ID: {0}")]
    DuplicateRunId(String),
    #[error("run references unknown parameter set: {0}")]
    UnknownParameterSet(String),
    #[error("completed run {0} is missing an outcome")]
    CompletedRunMissingOutcome(String),
    #[error("failed run {0} is missing an error message")]
    FailedRunMissingError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SimState;

    fn scenario(id: &str) -> Scenario {
        Scenario::new(id, "Test Scenario", SimState::zeros(2), 5)
    }

    #[test]
    fn test_bundle_validates_with_parameter_set_and_run() {
        let mut spec = ExperimentSpec::new("exp-1", "Baseline", scenario("baseline"));
        spec.default_monte_carlo = Some(MonteCarloConfig::with_seed(10, 5, 42));

        let mut bundle = ExperimentBundle::new(spec);
        bundle
            .add_parameter_set(ExperimentParameterSet::new(
                "variant-a",
                "Variant A",
                scenario("variant-a"),
            ))
            .unwrap();

        let run = ExperimentRunRecord::running("run-1", Some("variant-a"), Some(42))
            .mark_completed(ExperimentOutcome {
                monte_carlo: None,
                composure: None,
                replay: None,
                metadata: None,
            });
        bundle.record_run(run).unwrap();

        assert!(bundle.validate().is_ok());
    }

    #[test]
    fn test_duplicate_parameter_set_rejected() {
        let mut bundle = ExperimentBundle::new(ExperimentSpec::new(
            "exp-1",
            "Baseline",
            scenario("baseline"),
        ));
        bundle
            .add_parameter_set(ExperimentParameterSet::new(
                "variant-a",
                "Variant A",
                scenario("variant-a"),
            ))
            .unwrap();

        let err = bundle
            .add_parameter_set(ExperimentParameterSet::new(
                "variant-a",
                "Variant B",
                scenario("variant-b"),
            ))
            .unwrap_err();

        assert!(matches!(err, ExperimentError::DuplicateParameterSet(_)));
    }

    #[test]
    fn test_unknown_parameter_set_run_rejected() {
        let mut bundle = ExperimentBundle::new(ExperimentSpec::new(
            "exp-1",
            "Baseline",
            scenario("baseline"),
        ));

        let err = bundle
            .record_run(ExperimentRunRecord::running(
                "run-1",
                Some("missing-variant"),
                Some(7),
            ))
            .unwrap_err();

        assert!(matches!(err, ExperimentError::UnknownParameterSet(_)));
    }

    #[test]
    fn test_completed_run_requires_outcome() {
        let run = ExperimentRunRecord {
            run_id: "run-1".into(),
            parameter_set_id: None,
            status: ExperimentRunStatus::Completed,
            seed: Some(1),
            started_at_unix_s: 1,
            completed_at_unix_s: Some(2),
            outcome: None,
            error: None,
            metadata: None,
        };

        let err = run.validate().unwrap_err();
        assert!(matches!(
            err,
            ExperimentError::CompletedRunMissingOutcome(_)
        ));
    }
}
