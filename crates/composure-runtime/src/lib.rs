use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use composure_calibration::{CalibrationError, ObservedTrajectory};
use composure_core::{
    execute_experiment_spec, run_counterfactual, Action, ActionType, CounterfactualBranchInput,
    CounterfactualConfig, CounterfactualError, CounterfactualResult, ExperimentBundle,
    ExperimentError, ExperimentExecutionConfig, ExperimentExecutionError, ExperimentSpec,
    MonteCarloError, Scenario, ScenarioError, SensitivityError, SimState, Simulator,
    SweepDefinition,
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const MANIFEST_FILE: &str = "pack.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounterfactualDefinition {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub branch_state: SimState,
    pub baseline: CounterfactualBranchInput,
    pub candidate: CounterfactualBranchInput,
    pub config: CounterfactualConfig,
    pub runtime_model: PackRuntimeModel,
    pub metadata: Option<serde_json::Value>,
}

impl CounterfactualDefinition {
    pub fn validate(&self) -> Result<(), CounterfactualSpecError> {
        if self.id.trim().is_empty() {
            return Err(CounterfactualSpecError::EmptyCounterfactualId);
        }
        if self.name.trim().is_empty() {
            return Err(CounterfactualSpecError::EmptyCounterfactualName);
        }

        self.config
            .monte_carlo
            .validate()
            .map_err(CounterfactualSpecError::InvalidMonteCarlo)?;
        self.config
            .comparison
            .validate()
            .map_err(CounterfactualSpecError::InvalidComparison)?;

        if self.baseline.branch_id.trim().is_empty() {
            return Err(CounterfactualSpecError::EmptyBranchId { role: "baseline" });
        }
        if self.candidate.branch_id.trim().is_empty() {
            return Err(CounterfactualSpecError::EmptyBranchId { role: "candidate" });
        }
        if self.baseline.intervention_label.trim().is_empty() {
            return Err(CounterfactualSpecError::EmptyInterventionLabel { role: "baseline" });
        }
        if self.candidate.intervention_label.trim().is_empty() {
            return Err(CounterfactualSpecError::EmptyInterventionLabel { role: "candidate" });
        }
        if self.baseline.branch_id == self.candidate.branch_id {
            return Err(CounterfactualSpecError::DuplicateBranchId(
                self.baseline.branch_id.clone(),
            ));
        }

        validate_counterfactual_branch(
            &self.branch_state,
            &self.baseline,
            self.config.monte_carlo.time_steps,
            self.config.analysis_failure_threshold,
        )?;
        validate_counterfactual_branch(
            &self.branch_state,
            &self.candidate,
            self.config.monte_carlo.time_steps,
            self.config.analysis_failure_threshold,
        )?;

        self.runtime_model
            .validate(&counterfactual_runtime_scenario(
                &self.branch_state,
                self.config.monte_carlo.time_steps,
                self.config.analysis_failure_threshold,
            ))
            .map_err(CounterfactualSpecError::InvalidRuntimeModel)?;

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackDefinition {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub scenario: String,
    pub experiment_spec: Option<String>,
    pub sweep_definition: Option<String>,
    pub observed_trajectory: Option<String>,
    pub runtime_model: Option<PackRuntimeModel>,
    pub metadata: Option<serde_json::Value>,
}

impl PackDefinition {
    pub fn validate(&self) -> Result<(), PackError> {
        validate_non_empty(&self.id, PackError::EmptyPackId)?;
        validate_non_empty(&self.name, PackError::EmptyPackName)?;
        validate_non_empty(&self.scenario, PackError::EmptyScenarioPath)?;

        if let Some(path) = &self.experiment_spec {
            validate_non_empty(path, PackError::EmptyExperimentSpecPath)?;
        }
        if let Some(path) = &self.sweep_definition {
            validate_non_empty(path, PackError::EmptySweepDefinitionPath)?;
        }
        if let Some(path) = &self.observed_trajectory {
            validate_non_empty(path, PackError::EmptyObservedTrajectoryPath)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PackRuntimeModel {
    Linear(LinearRuntimeModel),
}

impl PackRuntimeModel {
    fn validate(&self, scenario: &Scenario) -> Result<(), PackError> {
        match self {
            Self::Linear(model) => model.validate(scenario),
        }
    }

    fn summary_label(&self) -> &'static str {
        match self {
            Self::Linear(_) => "linear",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinearRuntimeModel {
    pub dimensions: Vec<LinearRuntimeDimension>,
    pub action_type_scales: ActionTypeScales,
    pub noise_scale: f64,
    pub aggregate_weights: Option<Vec<f64>>,
}

impl LinearRuntimeModel {
    fn validate(&self, scenario: &Scenario) -> Result<(), PackError> {
        if self.dimensions.len() != scenario.initial_state.dimensions() {
            return Err(PackError::RuntimeDimensionsMismatch {
                expected: scenario.initial_state.dimensions(),
                actual: self.dimensions.len(),
            });
        }

        if !self.noise_scale.is_finite() {
            return Err(PackError::NonFiniteNoiseScale);
        }
        if self.noise_scale < 0.0 {
            return Err(PackError::NegativeNoiseScale(self.noise_scale));
        }

        self.action_type_scales.validate()?;

        if let Some(weights) = &self.aggregate_weights {
            if weights.len() != scenario.initial_state.dimensions() {
                return Err(PackError::AggregateWeightsMismatch {
                    expected: scenario.initial_state.dimensions(),
                    actual: weights.len(),
                });
            }
            if weights.iter().any(|value| !value.is_finite()) {
                return Err(PackError::NonFiniteAggregateWeights);
            }
        }

        for (index, dimension) in self.dimensions.iter().enumerate() {
            dimension.validate(index)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinearRuntimeDimension {
    pub drift: f64,
    pub action_gain: f64,
    pub memory_decay: f64,
    pub action_to_memory: f64,
    pub memory_to_state: f64,
    pub uncertainty_decay: f64,
    pub action_to_uncertainty: f64,
    pub min_value: f64,
    pub max_value: f64,
}

impl LinearRuntimeDimension {
    fn validate(&self, index: usize) -> Result<(), PackError> {
        for value in [
            self.drift,
            self.action_gain,
            self.memory_decay,
            self.action_to_memory,
            self.memory_to_state,
            self.uncertainty_decay,
            self.action_to_uncertainty,
            self.min_value,
            self.max_value,
        ] {
            if !value.is_finite() {
                return Err(PackError::NonFiniteRuntimeValue { index });
            }
        }

        if !(0.0..=1.0).contains(&self.memory_decay) {
            return Err(PackError::InvalidDecayValue {
                index,
                field: "memory_decay",
                value: self.memory_decay,
            });
        }
        if !(0.0..=1.0).contains(&self.uncertainty_decay) {
            return Err(PackError::InvalidDecayValue {
                index,
                field: "uncertainty_decay",
                value: self.uncertainty_decay,
            });
        }
        if self.min_value > self.max_value {
            return Err(PackError::InvalidValueBounds {
                index,
                min_value: self.min_value,
                max_value: self.max_value,
            });
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionTypeScales {
    pub intervention: f64,
    pub stressor_onset: f64,
    pub stressor_removal: f64,
    pub hold: f64,
    pub custom: BTreeMap<String, f64>,
}

impl ActionTypeScales {
    fn validate(&self) -> Result<(), PackError> {
        for value in [
            self.intervention,
            self.stressor_onset,
            self.stressor_removal,
            self.hold,
        ] {
            if !value.is_finite() {
                return Err(PackError::NonFiniteActionTypeScale);
            }
        }
        if self.custom.values().any(|value| !value.is_finite()) {
            return Err(PackError::NonFiniteActionTypeScale);
        }
        Ok(())
    }

    fn for_action(&self, action_type: &ActionType) -> f64 {
        match action_type {
            ActionType::Intervention => self.intervention,
            ActionType::StressorOnset => self.stressor_onset,
            ActionType::StressorRemoval => self.stressor_removal,
            ActionType::Hold => self.hold,
            ActionType::Custom(label) => self.custom.get(label).copied().unwrap_or(1.0),
        }
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

#[derive(Debug, Clone)]
pub struct CompiledCounterfactual {
    pub path: PathBuf,
    pub definition: CounterfactualDefinition,
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
            format!(
                "Runtime model: {}",
                self.definition
                    .runtime_model
                    .as_ref()
                    .map(PackRuntimeModel::summary_label)
                    .unwrap_or("none")
            ),
            format!(
                "Executable: {}",
                yes_no(self.definition.runtime_model.is_some() && self.experiment_spec.is_some())
            ),
        ]
        .join("\n")
    }
}

impl CompiledCounterfactual {
    pub fn summary(&self) -> String {
        [
            format!(
                "Counterfactual: {} ({})",
                self.definition.name, self.definition.id
            ),
            format!("Path: {}", self.path.display()),
            format!("Dimensions: {}", self.definition.branch_state.dimensions()),
            format!("Branch from t: {}", self.definition.branch_state.t),
            format!(
                "Time steps: {}",
                self.definition.config.monte_carlo.time_steps
            ),
            format!(
                "Baseline: {} ({})",
                self.definition.baseline.intervention_label, self.definition.baseline.branch_id
            ),
            format!(
                "Candidate: {} ({})",
                self.definition.candidate.intervention_label, self.definition.candidate.branch_id
            ),
            format!(
                "Comparison failure threshold: {:?}",
                self.definition.config.comparison.failure_threshold
            ),
            format!(
                "Analysis failure threshold: {:?}",
                self.definition.config.analysis_failure_threshold
            ),
            format!(
                "Runtime model: {}",
                self.definition.runtime_model.summary_label()
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
    compile_pack_with_mode(root, manifest_path, definition, PackLoadMode::Full)
}

pub fn load_counterfactual(
    path: impl AsRef<Path>,
) -> Result<CompiledCounterfactual, CounterfactualSpecError> {
    let path = path.as_ref().to_path_buf();
    let definition = read_counterfactual_json::<CounterfactualDefinition>(&path)?;
    definition.validate()?;
    Ok(CompiledCounterfactual { path, definition })
}

pub fn load_pack_for_run(path: impl AsRef<Path>) -> Result<CompiledPack, PackError> {
    let manifest_path = resolve_manifest_path(path.as_ref())?;
    let root = manifest_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    let definition = read_json::<PackDefinition>(&manifest_path)?;
    compile_pack_with_mode(root, manifest_path, definition, PackLoadMode::RuntimeOnly)
}

pub fn compile_pack(
    root: impl AsRef<Path>,
    manifest_path: impl AsRef<Path>,
    definition: PackDefinition,
) -> Result<CompiledPack, PackError> {
    compile_pack_with_mode(root, manifest_path, definition, PackLoadMode::Full)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PackLoadMode {
    Full,
    RuntimeOnly,
}

fn compile_pack_with_mode(
    root: impl AsRef<Path>,
    manifest_path: impl AsRef<Path>,
    definition: PackDefinition,
    mode: PackLoadMode,
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

    let sweep_definition = match mode {
        PackLoadMode::Full => definition
            .sweep_definition
            .as_ref()
            .map(|path| read_json::<SweepDefinition>(&root.join(path)))
            .transpose()?,
        PackLoadMode::RuntimeOnly => None,
    };

    let observed_trajectory = match mode {
        PackLoadMode::Full => definition
            .observed_trajectory
            .as_ref()
            .map(|path| read_json::<ObservedTrajectory>(&root.join(path)))
            .transpose()?,
        PackLoadMode::RuntimeOnly => None,
    };

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

    if let Some(runtime_model) = &definition.runtime_model {
        runtime_model.validate(&scenario)?;
    }

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

pub fn default_run_id(pack: &CompiledPack) -> String {
    format!("{}-run-1", pack.definition.id)
}

pub fn run_pack(
    pack: &CompiledPack,
    run_id: impl Into<String>,
    execution: &ExperimentExecutionConfig,
) -> Result<ExperimentBundle, PackRunError> {
    let runtime_model = pack
        .definition
        .runtime_model
        .as_ref()
        .ok_or(PackRunError::MissingRuntimeModel)?;
    let spec = pack
        .experiment_spec
        .as_ref()
        .ok_or(PackRunError::MissingExperimentSpec)?;

    let simulator = RuntimePackSimulator {
        model: runtime_model.clone(),
    };
    let run = execute_experiment_spec(run_id, &simulator, spec, execution)
        .map_err(PackRunError::Execute)?;

    let mut bundle = ExperimentBundle::new(spec.clone());
    bundle.record_run(run).map_err(PackRunError::RecordRun)?;
    Ok(bundle)
}

pub fn run_counterfactual_definition(
    definition: &CompiledCounterfactual,
) -> Result<CounterfactualResult, CounterfactualRunError> {
    let simulator = RuntimePackSimulator {
        model: definition.definition.runtime_model.clone(),
    };
    run_counterfactual(
        &simulator,
        &definition.definition.branch_state,
        &definition.definition.baseline,
        &definition.definition.candidate,
        &definition.definition.config,
    )
    .map_err(CounterfactualRunError::Execute)
}

#[derive(Debug, Clone)]
struct RuntimePackSimulator {
    model: PackRuntimeModel,
}

impl Simulator for RuntimePackSimulator {
    fn step(&self, state: &SimState, action: &Action, rng: &mut dyn rand::RngCore) -> SimState {
        match &self.model {
            PackRuntimeModel::Linear(model) => {
                let mut next = state.clone();
                next.t += 1;

                for (index, dimension) in model.dimensions.iter().enumerate() {
                    let targeted = action.dimension.map(|value| value == index).unwrap_or(true);
                    let signed_effect = if targeted {
                        normalized_action_magnitude(action)
                            * dimension.action_gain
                            * model.action_type_scales.for_action(&action.action_type)
                    } else {
                        0.0
                    };
                    let noise = (rng.gen::<f64>() - 0.5) * 2.0 * model.noise_scale;

                    next.z[index] = (state.z[index] + dimension.drift + signed_effect
                        - (state.m[index] * dimension.memory_to_state)
                        + noise)
                        .clamp(dimension.min_value, dimension.max_value);

                    next.m[index] = (state.m[index] * (1.0 - dimension.memory_decay)
                        + signed_effect.abs() * dimension.action_to_memory)
                        .clamp(0.0, 1.0);

                    next.u[index] = (state.u[index] * (1.0 - dimension.uncertainty_decay)
                        + signed_effect.abs() * dimension.action_to_uncertainty)
                        .clamp(0.0, 1.0);
                }

                next
            }
        }
    }

    fn health_index(&self, state: &SimState) -> f64 {
        match &self.model {
            PackRuntimeModel::Linear(model) => match &model.aggregate_weights {
                Some(weights) => {
                    let total_weight = weights.iter().sum::<f64>();
                    if total_weight.abs() < f64::EPSILON {
                        return state.default_health_index();
                    }
                    state
                        .z
                        .iter()
                        .zip(weights.iter())
                        .map(|(value, weight)| value * weight)
                        .sum::<f64>()
                        / total_weight
                }
                None => state.default_health_index(),
            },
        }
    }
}

fn normalized_action_magnitude(action: &Action) -> f64 {
    match action.action_type {
        ActionType::Hold => action.magnitude,
        ActionType::StressorRemoval => action.magnitude.abs(),
        _ => action.magnitude,
    }
}

fn resolve_manifest_path(path: &Path) -> Result<PathBuf, PackError> {
    if path.is_dir() {
        let manifest_path = path.join(MANIFEST_FILE);
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

fn read_counterfactual_json<T>(path: &Path) -> Result<T, CounterfactualSpecError>
where
    T: serde::de::DeserializeOwned,
{
    let raw = fs::read_to_string(path).map_err(|source| CounterfactualSpecError::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_str(&raw).map_err(|source| CounterfactualSpecError::ParseJson {
        path: path.to_path_buf(),
        source,
    })
}

fn counterfactual_runtime_scenario(
    branch_state: &SimState,
    time_steps: usize,
    failure_threshold: Option<f64>,
) -> Scenario {
    let mut scenario = Scenario::new(
        "counterfactual-runtime",
        "Counterfactual Runtime",
        branch_state.clone(),
        time_steps,
    );
    scenario.failure_threshold = failure_threshold;
    scenario
}

fn validate_counterfactual_branch(
    branch_state: &SimState,
    branch: &CounterfactualBranchInput,
    time_steps: usize,
    failure_threshold: Option<f64>,
) -> Result<(), CounterfactualSpecError> {
    let scenario = Scenario {
        id: format!("counterfactual-{}", branch.branch_id),
        name: format!("Counterfactual {}", branch.intervention_label),
        initial_state: branch_state.clone(),
        actions: branch.actions.clone(),
        time_steps,
        conditional_actions: branch.conditional_actions.clone(),
        failure_threshold,
        metadata: None,
    };
    scenario
        .validate()
        .map_err(|source| CounterfactualSpecError::InvalidBranchScenario {
            branch_id: branch.branch_id.clone(),
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

fn validate_non_empty(value: &str, error: PackError) -> Result<(), PackError> {
    if value.trim().is_empty() {
        return Err(error);
    }
    Ok(())
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
    #[error("experiment spec path cannot be empty")]
    EmptyExperimentSpecPath,
    #[error("sweep definition path cannot be empty")]
    EmptySweepDefinitionPath,
    #[error("observed trajectory path cannot be empty")]
    EmptyObservedTrajectoryPath,
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
    #[error("runtime dimensions must match state dimensions (expected {expected}, got {actual})")]
    RuntimeDimensionsMismatch { expected: usize, actual: usize },
    #[error(
        "runtime aggregate_weights must match state dimensions (expected {expected}, got {actual})"
    )]
    AggregateWeightsMismatch { expected: usize, actual: usize },
    #[error("runtime aggregate_weights must contain only finite values")]
    NonFiniteAggregateWeights,
    #[error("runtime action type scales must contain only finite values")]
    NonFiniteActionTypeScale,
    #[error("runtime noise_scale must be finite")]
    NonFiniteNoiseScale,
    #[error("runtime noise_scale must be >= 0, got {0}")]
    NegativeNoiseScale(f64),
    #[error("runtime dimension {index} contains a non-finite value")]
    NonFiniteRuntimeValue { index: usize },
    #[error("runtime dimension {index} field {field} must be in [0, 1], got {value}")]
    InvalidDecayValue {
        index: usize,
        field: &'static str,
        value: f64,
    },
    #[error(
        "runtime dimension {index} has invalid bounds: min_value={min_value}, max_value={max_value}"
    )]
    InvalidValueBounds {
        index: usize,
        min_value: f64,
        max_value: f64,
    },
}

#[derive(Debug, Error)]
pub enum PackRunError {
    #[error("pack does not define a runtime_model")]
    MissingRuntimeModel,
    #[error("pack runtime execution requires experiment-spec.json")]
    MissingExperimentSpec,
    #[error("pack execution failed: {0}")]
    Execute(ExperimentExecutionError),
    #[error("failed to record runtime run into bundle: {0}")]
    RecordRun(ExperimentError),
}

#[derive(Debug, Error)]
pub enum CounterfactualSpecError {
    #[error("counterfactual ID cannot be empty")]
    EmptyCounterfactualId,
    #[error("counterfactual name cannot be empty")]
    EmptyCounterfactualName,
    #[error("counterfactual {role} branch ID cannot be empty")]
    EmptyBranchId { role: &'static str },
    #[error("counterfactual {role} intervention label cannot be empty")]
    EmptyInterventionLabel { role: &'static str },
    #[error("counterfactual branch IDs must be unique, got {0}")]
    DuplicateBranchId(String),
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
    #[error("invalid Monte Carlo configuration: {0}")]
    InvalidMonteCarlo(MonteCarloError),
    #[error("invalid comparison configuration: {0}")]
    InvalidComparison(composure_core::CompareError),
    #[error("invalid counterfactual branch {branch_id}: {source}")]
    InvalidBranchScenario {
        branch_id: String,
        source: ScenarioError,
    },
    #[error("invalid runtime model: {0}")]
    InvalidRuntimeModel(PackError),
}

#[derive(Debug, Error)]
pub enum CounterfactualRunError {
    #[error("counterfactual execution failed: {0}")]
    Execute(CounterfactualError),
}

#[cfg(test)]
mod tests {
    use super::*;

    use composure_core::{
        Action, ActionType, ComparisonConfig, ConditionalActionRule, ConditionalTrigger,
        CounterfactualBranchInput, CounterfactualConfig, MonteCarloConfig, ParameterValue,
        SimState, SweepParameter, SweepStrategy,
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
        scenario.conditional_actions.push(ConditionalActionRule {
            id: "stabilize-readiness".into(),
            trigger: ConditionalTrigger::HealthIndexBelow { threshold: 0.5 },
            action: Action {
                dimension: Some(1),
                magnitude: 0.12,
                action_type: ActionType::Intervention,
                metadata: None,
            },
            delay_steps: 1,
            cooldown_steps: 2,
            priority: 1,
            max_fires: Some(1),
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

    fn sample_definition(with_runtime: bool) -> PackDefinition {
        PackDefinition {
            id: "health-pack".into(),
            name: "Health Pack".into(),
            description: None,
            scenario: "scenario.json".into(),
            experiment_spec: Some("experiment-spec.json".into()),
            sweep_definition: Some("sweep-definition.json".into()),
            observed_trajectory: Some("observed-trajectory.json".into()),
            runtime_model: with_runtime.then(|| {
                PackRuntimeModel::Linear(LinearRuntimeModel {
                    dimensions: vec![
                        LinearRuntimeDimension {
                            drift: 0.01,
                            action_gain: 0.08,
                            memory_decay: 0.1,
                            action_to_memory: 0.06,
                            memory_to_state: 0.04,
                            uncertainty_decay: 0.05,
                            action_to_uncertainty: 0.2,
                            min_value: 0.0,
                            max_value: 1.0,
                        },
                        LinearRuntimeDimension {
                            drift: 0.015,
                            action_gain: 0.06,
                            memory_decay: 0.08,
                            action_to_memory: 0.05,
                            memory_to_state: 0.03,
                            uncertainty_decay: 0.05,
                            action_to_uncertainty: 0.2,
                            min_value: 0.0,
                            max_value: 1.0,
                        },
                    ],
                    action_type_scales: ActionTypeScales {
                        intervention: 1.0,
                        stressor_onset: 1.0,
                        stressor_removal: 0.8,
                        hold: 0.0,
                        custom: BTreeMap::new(),
                    },
                    noise_scale: 0.01,
                    aggregate_weights: Some(vec![0.7, 0.3]),
                })
            }),
            metadata: None,
        }
    }

    fn write_pack(dir: &Path, with_runtime: bool) {
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
            serde_json::to_string_pretty(&sample_definition(with_runtime)).unwrap(),
        )
        .unwrap();
    }

    fn sample_counterfactual_definition() -> CounterfactualDefinition {
        CounterfactualDefinition {
            id: "cf-1".into(),
            name: "Recovery branch".into(),
            description: None,
            branch_state: SimState::new(vec![0.4, 0.5], vec![0.1, 0.2], vec![0.2, 0.2]),
            baseline: CounterfactualBranchInput {
                branch_id: "baseline".into(),
                intervention_label: "No change".into(),
                actions: vec![
                    Action {
                        dimension: Some(0),
                        magnitude: 0.0,
                        action_type: ActionType::Hold,
                        metadata: None,
                    };
                    4
                ],
                conditional_actions: Vec::new(),
            },
            candidate: CounterfactualBranchInput {
                branch_id: "candidate".into(),
                intervention_label: "Recovery".into(),
                actions: vec![
                    Action {
                        dimension: Some(0),
                        magnitude: 0.2,
                        action_type: ActionType::Intervention,
                        metadata: None,
                    };
                    4
                ],
                conditional_actions: vec![ConditionalActionRule {
                    id: "stabilize".into(),
                    trigger: ConditionalTrigger::HealthIndexBelow { threshold: 0.45 },
                    action: Action {
                        dimension: Some(1),
                        magnitude: 0.1,
                        action_type: ActionType::Intervention,
                        metadata: None,
                    },
                    delay_steps: 1,
                    cooldown_steps: 2,
                    priority: 1,
                    max_fires: Some(1),
                }],
            },
            config: CounterfactualConfig {
                monte_carlo: MonteCarloConfig::with_seed(6, 4, 19),
                execution: ExperimentExecutionConfig {
                    retain_paths: true,
                    analyze_composure: true,
                },
                comparison: ComparisonConfig {
                    failure_threshold: Some(0.45),
                    ..ComparisonConfig::default()
                },
                analysis_failure_threshold: Some(0.45),
            },
            runtime_model: sample_definition(true).runtime_model.unwrap(),
            metadata: None,
        }
    }

    fn write_counterfactual(dir: &Path) -> PathBuf {
        let path = dir.join("counterfactual.json");
        fs::write(
            &path,
            serde_json::to_string_pretty(&sample_counterfactual_definition()).unwrap(),
        )
        .unwrap();
        path
    }

    #[test]
    fn test_load_pack_reads_and_validates_manifest() {
        let dir = temp_pack_dir("valid");
        write_pack(&dir, false);

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
        write_pack(&dir, false);

        let pack = load_pack(&dir).unwrap();
        assert_eq!(pack.definition.id, "health-pack");
        assert_eq!(pack.manifest_path, dir.join("pack.json"));
    }

    #[test]
    fn test_load_pack_rejects_mismatched_observed_length() {
        let dir = temp_pack_dir("observed-mismatch");
        write_pack(&dir, false);

        let mut observed: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(dir.join("observed-trajectory.json")).unwrap(),
        )
        .unwrap();
        observed["values"] = serde_json::json!([0.4, 0.45, 0.5]);
        fs::write(
            dir.join("observed-trajectory.json"),
            serde_json::to_string_pretty(&observed).unwrap(),
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

    #[test]
    fn test_run_pack_returns_bundle() {
        let dir = temp_pack_dir("run-pack");
        write_pack(&dir, true);

        let pack = load_pack_for_run(dir.join("pack.json")).unwrap();
        let bundle = run_pack(
            &pack,
            "health-pack-run-1",
            &ExperimentExecutionConfig::default(),
        )
        .unwrap();

        assert_eq!(bundle.spec.id, "exp-1");
        assert_eq!(bundle.runs.len(), 1);
        assert_eq!(bundle.runs[0].run_id, "health-pack-run-1");
        assert!(bundle.runs[0]
            .outcome
            .as_ref()
            .unwrap()
            .monte_carlo
            .is_some());
    }

    #[test]
    fn test_load_pack_for_run_skips_unused_artifacts() {
        let dir = temp_pack_dir("run-pack-runtime-only");
        write_pack(&dir, true);
        fs::write(dir.join("sweep-definition.json"), "{not-json").unwrap();
        fs::write(dir.join("observed-trajectory.json"), "{not-json").unwrap();

        let pack = load_pack_for_run(dir.join("pack.json")).unwrap();
        assert!(pack.experiment_spec.is_some());
        assert!(pack.sweep_definition.is_none());
        assert!(pack.observed_trajectory.is_none());
    }

    #[test]
    fn test_load_pack_for_run_still_requires_valid_runtime_inputs() {
        let dir = temp_pack_dir("run-pack-runtime-dims");
        write_pack(&dir, true);

        let mut manifest: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(dir.join("pack.json")).unwrap()).unwrap();
        manifest["runtime_model"]["dimensions"] = serde_json::json!([{
            "drift": 0.01,
            "action_gain": 0.08,
            "memory_decay": 0.1,
            "action_to_memory": 0.06,
            "memory_to_state": 0.04,
            "uncertainty_decay": 0.05,
            "action_to_uncertainty": 0.2,
            "min_value": 0.0,
            "max_value": 1.0
        }]);
        fs::write(
            dir.join("pack.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let err = load_pack_for_run(dir.join("pack.json")).unwrap_err();
        assert!(matches!(
            err,
            PackError::RuntimeDimensionsMismatch {
                expected: 2,
                actual: 1
            }
        ));
    }

    #[test]
    fn test_load_counterfactual_reads_and_validates_definition() {
        let dir = temp_pack_dir("counterfactual-valid");
        let path = write_counterfactual(&dir);

        let counterfactual = load_counterfactual(&path).unwrap();
        assert_eq!(counterfactual.definition.id, "cf-1");
        assert_eq!(counterfactual.definition.baseline.branch_id, "baseline");
        assert_eq!(counterfactual.definition.candidate.branch_id, "candidate");
        assert_eq!(counterfactual.definition.config.monte_carlo.time_steps, 4);
    }

    #[test]
    fn test_load_counterfactual_rejects_invalid_runtime_dimensions() {
        let dir = temp_pack_dir("counterfactual-runtime-dims");
        let path = write_counterfactual(&dir);
        let mut definition: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        definition["runtime_model"]["dimensions"] = serde_json::json!([{
            "drift": 0.01,
            "action_gain": 0.08,
            "memory_decay": 0.1,
            "action_to_memory": 0.06,
            "memory_to_state": 0.04,
            "uncertainty_decay": 0.05,
            "action_to_uncertainty": 0.2,
            "min_value": 0.0,
            "max_value": 1.0
        }]);
        fs::write(&path, serde_json::to_string_pretty(&definition).unwrap()).unwrap();

        let err = load_counterfactual(&path).unwrap_err();
        assert!(matches!(
            err,
            CounterfactualSpecError::InvalidRuntimeModel(PackError::RuntimeDimensionsMismatch {
                expected: 2,
                actual: 1,
            })
        ));
    }

    #[test]
    fn test_run_counterfactual_definition_returns_result() {
        let dir = temp_pack_dir("counterfactual-run");
        let path = write_counterfactual(&dir);

        let counterfactual = load_counterfactual(&path).unwrap();
        let result = run_counterfactual_definition(&counterfactual).unwrap();

        assert_eq!(result.baseline.branch_id, "baseline");
        assert_eq!(result.candidate.branch_id, "candidate");
        assert!(result.comparison.metrics.end_delta > 0.0);
        assert!(result.baseline.outcome.monte_carlo.is_some());
        assert!(result.candidate.outcome.monte_carlo.is_some());
    }
}
