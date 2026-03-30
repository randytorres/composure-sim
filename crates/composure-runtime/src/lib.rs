use std::{
    fs,
    path::{Path, PathBuf},
};

use composure_calibration::{CalibrationError, ObservedTrajectory};
use composure_core::{
    ExperimentError, ExperimentSpec, Scenario, ScenarioError, SensitivityError, SweepDefinition,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackDefinition {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub scenario: String,
    pub experiment_spec: Option<String>,
    pub sweep_definition: Option<String>,
    pub observed_trajectory: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

impl PackDefinition {
    pub fn validate(&self) -> Result<(), PackError> {
        if self.id.trim().is_empty() {
            return Err(PackError::EmptyPackId);
        }
        if self.name.trim().is_empty() {
            return Err(PackError::EmptyPackName);
        }
        if self.scenario.trim().is_empty() {
            return Err(PackError::EmptyScenarioPath);
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct CompiledPack {
    pub root: PathBuf,
    pub manifest_path: PathBuf,
    pub definition: PackDefinition,
    pub scenario: Scenario,
    pub experiment_spec: Option<ExperimentSpec>,
    pub sweep_definition: Option<SweepDefinition>,
    pub observed_trajectory: Option<ObservedTrajectory>,
    pub dimension_labels: Vec<String>,
}

impl CompiledPack {
    pub fn summary(&self) -> String {
        [
            format!("Pack: {} ({})", self.definition.name, self.definition.id),
            format!("Root: {}", self.root.display()),
            format!("Scenario: {} ({})", self.scenario.name, self.scenario.id),
            format!("Dimensions: {}", self.scenario.initial_state.dimensions()),
            format!("Time steps: {}", self.scenario.time_steps),
            format!(
                "Dimension labels: {}",
                if self.dimension_labels.is_empty() {
                    "none".into()
                } else {
                    self.dimension_labels.join(", ")
                }
            ),
            format!(
                "Experiment spec: {}",
                yes_no(self.experiment_spec.is_some())
            ),
            format!(
                "Sweep definition: {}",
                yes_no(self.sweep_definition.is_some())
            ),
            format!(
                "Observed trajectory: {}",
                yes_no(self.observed_trajectory.is_some())
            ),
        ]
        .join("\n")
    }
}

pub fn load_pack(path: impl AsRef<Path>) -> Result<CompiledPack, PackError> {
    let manifest_path = resolve_manifest_path(path.as_ref())?;
    let root = manifest_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    let definition = read_json::<PackDefinition>(&manifest_path)?;
    compile_pack(root, manifest_path, definition)
}

pub fn compile_pack(
    root: impl AsRef<Path>,
    manifest_path: impl AsRef<Path>,
    definition: PackDefinition,
) -> Result<CompiledPack, PackError> {
    let root = root.as_ref().to_path_buf();
    let manifest_path = manifest_path.as_ref().to_path_buf();
    definition.validate()?;

    let scenario = read_json::<Scenario>(&root.join(&definition.scenario))?;
    scenario.validate().map_err(PackError::InvalidScenario)?;

    let experiment_spec = definition
        .experiment_spec
        .as_ref()
        .map(|path| read_json::<ExperimentSpec>(&root.join(path)))
        .transpose()?;

    let sweep_definition = definition
        .sweep_definition
        .as_ref()
        .map(|path| read_json::<SweepDefinition>(&root.join(path)))
        .transpose()?;

    let observed_trajectory = definition
        .observed_trajectory
        .as_ref()
        .map(|path| read_json::<ObservedTrajectory>(&root.join(path)))
        .transpose()?;

    if let Some(spec) = &experiment_spec {
        spec.validate().map_err(PackError::InvalidExperimentSpec)?;
        if !json_equal(&spec.scenario, &scenario) {
            return Err(PackError::ScenarioMismatch {
                left: "scenario.json".into(),
                right: "experiment-spec.json".into(),
            });
        }
        if let Some(config) = &spec.default_monte_carlo {
            if config.time_steps != scenario.time_steps {
                return Err(PackError::MonteCarloTimeStepsMismatch {
                    expected: scenario.time_steps,
                    actual: config.time_steps,
                });
            }
        }
    }

    if let Some(definition) = &sweep_definition {
        definition.validate().map_err(PackError::InvalidSweep)?;
    }

    if let Some(observed) = &observed_trajectory {
        observed
            .validate()
            .map_err(PackError::InvalidObservedTrajectory)?;
        if observed.values.len() != scenario.time_steps {
            return Err(PackError::ObservedLengthMismatch {
                expected: scenario.time_steps,
                actual: observed.values.len(),
            });
        }
    }

    let dimension_labels = extract_dimension_labels(&scenario)?;

    Ok(CompiledPack {
        root,
        manifest_path,
        definition,
        scenario,
        experiment_spec,
        sweep_definition,
        observed_trajectory,
        dimension_labels,
    })
}

fn resolve_manifest_path(path: &Path) -> Result<PathBuf, PackError> {
    if path.is_dir() {
        let manifest_path = path.join("pack.json");
        if !manifest_path.exists() {
            return Err(PackError::MissingPackManifest(path.to_path_buf()));
        }
        Ok(manifest_path)
    } else {
        Ok(path.to_path_buf())
    }
}

fn extract_dimension_labels(scenario: &Scenario) -> Result<Vec<String>, PackError> {
    let Some(metadata) = &scenario.metadata else {
        return Ok(Vec::new());
    };

    let Some(labels) = metadata.get("dimension_labels") else {
        return Ok(Vec::new());
    };

    let labels = labels
        .as_array()
        .ok_or(PackError::InvalidDimensionLabelsShape)?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_string)
                .ok_or(PackError::InvalidDimensionLabelValue)
        })
        .collect::<Result<Vec<_>, _>>()?;

    if labels.len() != scenario.initial_state.dimensions() {
        return Err(PackError::DimensionLabelsMismatch {
            expected: scenario.initial_state.dimensions(),
            actual: labels.len(),
        });
    }

    Ok(labels)
}

fn read_json<T>(path: &Path) -> Result<T, PackError>
where
    T: serde::de::DeserializeOwned,
{
    let raw = fs::read_to_string(path).map_err(|source| PackError::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_str(&raw).map_err(|source| PackError::ParseJson {
        path: path.to_path_buf(),
        source,
    })
}

fn json_equal<T>(left: &T, right: &T) -> bool
where
    T: Serialize,
{
    match (serde_json::to_value(left), serde_json::to_value(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

#[derive(Debug, Error)]
pub enum PackError {
    #[error("pack ID cannot be empty")]
    EmptyPackId,
    #[error("pack name cannot be empty")]
    EmptyPackName,
    #[error("scenario path cannot be empty")]
    EmptyScenarioPath,
    #[error("directory {0} does not contain pack.json")]
    MissingPackManifest(PathBuf),
    #[error("failed to read {path}: {source}")]
    ReadFile {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to parse JSON in {path}: {source}")]
    ParseJson {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("invalid scenario: {0}")]
    InvalidScenario(ScenarioError),
    #[error("invalid experiment spec: {0}")]
    InvalidExperimentSpec(ExperimentError),
    #[error("invalid sweep definition: {0}")]
    InvalidSweep(SensitivityError),
    #[error("invalid observed trajectory: {0}")]
    InvalidObservedTrajectory(CalibrationError),
    #[error("{left} and {right} must describe the same scenario")]
    ScenarioMismatch { left: String, right: String },
    #[error(
        "default Monte Carlo time steps must match scenario time steps (expected {expected}, got {actual})"
    )]
    MonteCarloTimeStepsMismatch { expected: usize, actual: usize },
    #[error(
        "observed trajectory length must match scenario time steps (expected {expected}, got {actual})"
    )]
    ObservedLengthMismatch { expected: usize, actual: usize },
    #[error("dimension_labels metadata must be an array of strings")]
    InvalidDimensionLabelsShape,
    #[error("dimension_labels metadata must contain only strings")]
    InvalidDimensionLabelValue,
    #[error(
        "dimension_labels count must match state dimensions (expected {expected}, got {actual})"
    )]
    DimensionLabelsMismatch { expected: usize, actual: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    use composure_core::{
        Action, ActionType, MonteCarloConfig, ParameterValue, SimState, SweepParameter,
        SweepStrategy,
    };
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_pack_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("composure-runtime-{name}-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn sample_scenario() -> Scenario {
        let mut scenario = Scenario::new("health", "Health", SimState::zeros(2), 4);
        scenario.initial_state = SimState::new(vec![0.4, 0.5], vec![0.1, 0.2], vec![0.2, 0.2]);
        scenario.failure_threshold = Some(0.45);
        scenario.actions.push(Action {
            dimension: Some(0),
            magnitude: 0.2,
            action_type: ActionType::Intervention,
            metadata: None,
        });
        scenario.metadata = Some(serde_json::json!({
            "dimension_labels": ["sleep", "readiness"]
        }));
        scenario
    }

    fn sample_spec(scenario: &Scenario) -> ExperimentSpec {
        let mut spec = ExperimentSpec::new("exp-1", "Experiment", scenario.clone());
        spec.default_monte_carlo = Some(MonteCarloConfig::with_seed(8, scenario.time_steps, 7));
        spec
    }

    fn sample_sweep() -> SweepDefinition {
        let mut definition = SweepDefinition::new("sweep-1", "Sweep");
        definition.strategy = SweepStrategy::Grid;
        definition.parameters.push(SweepParameter {
            name: "dose".into(),
            values: vec![ParameterValue::Int(1)],
        });
        definition
    }

    fn sample_observed() -> ObservedTrajectory {
        ObservedTrajectory::new("obs-1", "Observed", vec![0.4, 0.45, 0.5, 0.55])
    }

    #[test]
    fn test_load_pack_reads_and_validates_manifest() {
        let dir = temp_pack_dir("valid");
        let scenario = sample_scenario();
        let spec = sample_spec(&scenario);
        let sweep = sample_sweep();
        let observed = sample_observed();

        fs::write(
            dir.join("scenario.json"),
            serde_json::to_string_pretty(&scenario).unwrap(),
        )
        .unwrap();
        fs::write(
            dir.join("experiment-spec.json"),
            serde_json::to_string_pretty(&spec).unwrap(),
        )
        .unwrap();
        fs::write(
            dir.join("sweep-definition.json"),
            serde_json::to_string_pretty(&sweep).unwrap(),
        )
        .unwrap();
        fs::write(
            dir.join("observed-trajectory.json"),
            serde_json::to_string_pretty(&observed).unwrap(),
        )
        .unwrap();
        fs::write(
            dir.join("pack.json"),
            serde_json::to_string_pretty(&PackDefinition {
                id: "health-pack".into(),
                name: "Health Pack".into(),
                description: None,
                scenario: "scenario.json".into(),
                experiment_spec: Some("experiment-spec.json".into()),
                sweep_definition: Some("sweep-definition.json".into()),
                observed_trajectory: Some("observed-trajectory.json".into()),
                metadata: None,
            })
            .unwrap(),
        )
        .unwrap();

        let pack = load_pack(dir.join("pack.json")).unwrap();
        assert_eq!(pack.definition.id, "health-pack");
        assert_eq!(pack.dimension_labels, vec!["sleep", "readiness"]);
        assert!(pack.experiment_spec.is_some());
        assert!(pack.sweep_definition.is_some());
        assert!(pack.observed_trajectory.is_some());
    }

    #[test]
    fn test_load_pack_accepts_directory_root() {
        let dir = temp_pack_dir("directory");
        let scenario = sample_scenario();
        let spec = sample_spec(&scenario);
        let sweep = sample_sweep();

        fs::write(
            dir.join("scenario.json"),
            serde_json::to_string_pretty(&scenario).unwrap(),
        )
        .unwrap();
        fs::write(
            dir.join("experiment-spec.json"),
            serde_json::to_string_pretty(&spec).unwrap(),
        )
        .unwrap();
        fs::write(
            dir.join("sweep-definition.json"),
            serde_json::to_string_pretty(&sweep).unwrap(),
        )
        .unwrap();
        fs::write(
            dir.join("pack.json"),
            serde_json::to_string_pretty(&PackDefinition {
                id: "health-pack".into(),
                name: "Health Pack".into(),
                description: None,
                scenario: "scenario.json".into(),
                experiment_spec: Some("experiment-spec.json".into()),
                sweep_definition: Some("sweep-definition.json".into()),
                observed_trajectory: None,
                metadata: None,
            })
            .unwrap(),
        )
        .unwrap();

        let pack = load_pack(&dir).unwrap();
        assert_eq!(pack.definition.id, "health-pack");
        assert_eq!(pack.manifest_path, dir.join("pack.json"));
    }

    #[test]
    fn test_load_pack_rejects_mismatched_observed_length() {
        let dir = temp_pack_dir("observed-mismatch");
        let scenario = sample_scenario();
        let spec = sample_spec(&scenario);
        let sweep = sample_sweep();
        let mut observed = sample_observed();
        observed.values.pop();

        fs::write(
            dir.join("scenario.json"),
            serde_json::to_string_pretty(&scenario).unwrap(),
        )
        .unwrap();
        fs::write(
            dir.join("experiment-spec.json"),
            serde_json::to_string_pretty(&spec).unwrap(),
        )
        .unwrap();
        fs::write(
            dir.join("sweep-definition.json"),
            serde_json::to_string_pretty(&sweep).unwrap(),
        )
        .unwrap();
        fs::write(
            dir.join("observed-trajectory.json"),
            serde_json::to_string_pretty(&observed).unwrap(),
        )
        .unwrap();
        fs::write(
            dir.join("pack.json"),
            serde_json::to_string_pretty(&PackDefinition {
                id: "health-pack".into(),
                name: "Health Pack".into(),
                description: None,
                scenario: "scenario.json".into(),
                experiment_spec: Some("experiment-spec.json".into()),
                sweep_definition: Some("sweep-definition.json".into()),
                observed_trajectory: Some("observed-trajectory.json".into()),
                metadata: None,
            })
            .unwrap(),
        )
        .unwrap();

        let err = load_pack(dir.join("pack.json")).unwrap_err();
        assert!(matches!(
            err,
            PackError::ObservedLengthMismatch {
                expected: 4,
                actual: 3
            }
        ));
    }
}
