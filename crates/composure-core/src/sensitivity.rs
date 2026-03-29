use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Portable parameter value used by sweep definitions and analyzed samples.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ParameterValue {
    Bool(bool),
    Int(i64),
    Float(String),
    Text(String),
}

impl ParameterValue {
    pub fn from_f64(value: f64) -> Self {
        Self::Float(format!("{value:.12}"))
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Bool(value) => Some(if *value { 1.0 } else { 0.0 }),
            Self::Int(value) => Some(*value as f64),
            Self::Float(value) => value.parse::<f64>().ok(),
            Self::Text(_) => None,
        }
    }
}

/// A single sweep parameter with an explicit set of candidate values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepParameter {
    pub name: String,
    pub values: Vec<ParameterValue>,
}

/// Strategy used to enumerate parameter combinations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SweepStrategy {
    Grid,
}

/// Serializable sweep definition that downstream tooling can expand into concrete cases.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepDefinition {
    pub id: String,
    pub name: String,
    pub parameters: Vec<SweepParameter>,
    pub strategy: SweepStrategy,
    pub metadata: Option<serde_json::Value>,
}

impl SweepDefinition {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            parameters: Vec::new(),
            strategy: SweepStrategy::Grid,
            metadata: None,
        }
    }

    pub fn validate(&self) -> Result<(), SensitivityError> {
        validate_identifier(&self.id, "sweep definition ID")?;
        validate_identifier(&self.name, "sweep definition name")?;
        if self.parameters.is_empty() {
            return Err(SensitivityError::EmptySweepParameters);
        }

        let mut seen = BTreeSet::new();
        for parameter in &self.parameters {
            validate_identifier(&parameter.name, "parameter name")?;
            if parameter.values.is_empty() {
                return Err(SensitivityError::EmptyParameterValues(
                    parameter.name.clone(),
                ));
            }
            if !seen.insert(parameter.name.as_str()) {
                return Err(SensitivityError::DuplicateParameter(parameter.name.clone()));
            }
            for value in &parameter.values {
                if let ParameterValue::Float(raw) = value {
                    raw.parse::<f64>()
                        .map_err(|_| SensitivityError::InvalidFloatValue(raw.clone()))?;
                }
            }
        }

        Ok(())
    }
}

/// One concrete case generated from a sweep definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepCase {
    pub case_id: String,
    pub parameters: BTreeMap<String, ParameterValue>,
}

/// Expand a sweep definition into concrete cases.
pub fn generate_sweep_cases(
    definition: &SweepDefinition,
) -> Result<Vec<SweepCase>, SensitivityError> {
    definition.validate()?;

    match definition.strategy {
        SweepStrategy::Grid => {
            let mut cases = vec![BTreeMap::new()];

            for parameter in &definition.parameters {
                let mut next = Vec::with_capacity(cases.len() * parameter.values.len());
                for existing in &cases {
                    for value in &parameter.values {
                        let mut map = existing.clone();
                        map.insert(parameter.name.clone(), value.clone());
                        next.push(map);
                    }
                }
                cases = next;
            }

            Ok(cases
                .into_iter()
                .enumerate()
                .map(|(idx, parameters)| SweepCase {
                    case_id: format!("{}-{}", definition.id, idx + 1),
                    parameters,
                })
                .collect())
        }
    }
}

/// One evaluated sweep sample with a scalar objective.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepSample {
    pub case_id: String,
    pub parameters: BTreeMap<String, ParameterValue>,
    pub objective: f64,
    pub metadata: Option<serde_json::Value>,
}

/// Configuration for post-run sensitivity analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitivityConfig {
    /// Small tolerance used for zero-variance and direction classification.
    pub epsilon: f64,
}

impl Default for SensitivityConfig {
    fn default() -> Self {
        Self { epsilon: 1e-9 }
    }
}

impl SensitivityConfig {
    pub fn validate(&self) -> Result<(), SensitivityError> {
        if !self.epsilon.is_finite() || self.epsilon < 0.0 {
            return Err(SensitivityError::InvalidEpsilon(self.epsilon));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SensitivityDirection {
    Positive,
    Negative,
    Mixed,
    Neutral,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectiveSummary {
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub best_case_id: String,
    pub worst_case_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NumericSensitivityStats {
    /// Pearson correlation between parameter value and objective.
    pub correlation: f64,
    /// Least-squares slope of objective vs parameter.
    pub slope: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoricalBucketSummary {
    pub value: ParameterValue,
    pub sample_count: usize,
    pub mean_objective: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoricalSensitivityStats {
    /// Difference between best and worst category means.
    pub range: f64,
    pub buckets: Vec<CategoricalBucketSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SensitivityKind {
    Numeric(NumericSensitivityStats),
    Categorical(CategoricalSensitivityStats),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterSensitivity {
    pub parameter: String,
    /// Higher score means stronger relationship with the objective.
    pub score: f64,
    pub direction: SensitivityDirection,
    pub kind: SensitivityKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitivityReport {
    pub sample_count: usize,
    pub objective: ObjectiveSummary,
    pub rankings: Vec<ParameterSensitivity>,
    pub config: SensitivityConfig,
}

/// Analyze completed sweep samples and rank parameters by apparent influence.
pub fn analyze_sensitivity(
    samples: &[SweepSample],
    config: &SensitivityConfig,
) -> Result<SensitivityReport, SensitivityError> {
    config.validate()?;
    if samples.is_empty() {
        return Err(SensitivityError::EmptySamples);
    }

    let expected_keys = sample_keys(&samples[0].parameters);
    if expected_keys.is_empty() {
        return Err(SensitivityError::EmptySweepParameters);
    }

    for sample in samples {
        validate_identifier(&sample.case_id, "sample case ID")?;
        if !sample.objective.is_finite() {
            return Err(SensitivityError::NonFiniteObjective {
                case_id: sample.case_id.clone(),
                objective: sample.objective,
            });
        }
        let keys = sample_keys(&sample.parameters);
        if keys != expected_keys {
            return Err(SensitivityError::InconsistentSampleParameters {
                case_id: sample.case_id.clone(),
            });
        }
    }

    let objective = summarize_objective(samples);
    let objective_span = objective.max - objective.min;
    let objectives: Vec<f64> = samples.iter().map(|sample| sample.objective).collect();

    let mut rankings = Vec::with_capacity(expected_keys.len());

    for parameter in &expected_keys {
        let values: Vec<&ParameterValue> = samples
            .iter()
            .map(|sample| {
                sample
                    .parameters
                    .get(parameter)
                    .expect("parameter keys are consistent")
            })
            .collect();

        let sensitivity = if values.iter().all(|value| value.as_f64().is_some()) {
            let xs: Vec<f64> = values
                .iter()
                .map(|value| value.as_f64().expect("already checked"))
                .collect();
            analyze_numeric_parameter(parameter, &xs, &objectives, config)
        } else {
            analyze_categorical_parameter(parameter, &values, &objectives, objective_span, config)
        };

        rankings.push(sensitivity);
    }

    rankings.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(SensitivityReport {
        sample_count: samples.len(),
        objective,
        rankings,
        config: config.clone(),
    })
}

fn analyze_numeric_parameter(
    parameter: &str,
    xs: &[f64],
    ys: &[f64],
    config: &SensitivityConfig,
) -> ParameterSensitivity {
    let mean_x = mean(xs);
    let mean_y = mean(ys);
    let var_x = xs.iter().map(|x| (x - mean_x).powi(2)).sum::<f64>() / xs.len() as f64;
    let var_y = ys.iter().map(|y| (y - mean_y).powi(2)).sum::<f64>() / ys.len() as f64;
    let covariance = xs
        .iter()
        .zip(ys.iter())
        .map(|(x, y)| (x - mean_x) * (y - mean_y))
        .sum::<f64>()
        / xs.len() as f64;

    let correlation = if var_x <= config.epsilon || var_y <= config.epsilon {
        0.0
    } else {
        covariance / (var_x.sqrt() * var_y.sqrt())
    };
    let slope = if var_x <= config.epsilon {
        0.0
    } else {
        covariance / var_x
    };

    let direction = if correlation > config.epsilon {
        SensitivityDirection::Positive
    } else if correlation < -config.epsilon {
        SensitivityDirection::Negative
    } else {
        SensitivityDirection::Neutral
    };

    ParameterSensitivity {
        parameter: parameter.to_string(),
        score: correlation.abs(),
        direction,
        kind: SensitivityKind::Numeric(NumericSensitivityStats { correlation, slope }),
    }
}

fn analyze_categorical_parameter(
    parameter: &str,
    values: &[&ParameterValue],
    objectives: &[f64],
    objective_span: f64,
    config: &SensitivityConfig,
) -> ParameterSensitivity {
    let mut grouped: BTreeMap<ParameterValue, Vec<f64>> = BTreeMap::new();
    for (value, objective) in values.iter().zip(objectives.iter()) {
        grouped
            .entry((*value).clone())
            .or_default()
            .push(*objective);
    }

    let mut buckets: Vec<CategoricalBucketSummary> = grouped
        .into_iter()
        .map(|(value, bucket)| CategoricalBucketSummary {
            value,
            sample_count: bucket.len(),
            mean_objective: mean(&bucket),
        })
        .collect();
    buckets.sort_by(|a, b| a.value.cmp(&b.value));

    let min_mean = buckets
        .iter()
        .map(|bucket| bucket.mean_objective)
        .fold(f64::INFINITY, f64::min);
    let max_mean = buckets
        .iter()
        .map(|bucket| bucket.mean_objective)
        .fold(f64::NEG_INFINITY, f64::max);
    let range = max_mean - min_mean;

    let score = if objective_span <= config.epsilon {
        0.0
    } else {
        (range / objective_span).clamp(0.0, 1.0)
    };

    let direction = if range <= config.epsilon {
        SensitivityDirection::Neutral
    } else {
        SensitivityDirection::Mixed
    };

    ParameterSensitivity {
        parameter: parameter.to_string(),
        score,
        direction,
        kind: SensitivityKind::Categorical(CategoricalSensitivityStats { range, buckets }),
    }
}

fn summarize_objective(samples: &[SweepSample]) -> ObjectiveSummary {
    let best = samples
        .iter()
        .max_by(|a, b| {
            a.objective
                .partial_cmp(&b.objective)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .expect("samples is non-empty");
    let worst = samples
        .iter()
        .min_by(|a, b| {
            a.objective
                .partial_cmp(&b.objective)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .expect("samples is non-empty");

    ObjectiveSummary {
        min: worst.objective,
        max: best.objective,
        mean: mean(
            &samples
                .iter()
                .map(|sample| sample.objective)
                .collect::<Vec<_>>(),
        ),
        best_case_id: best.case_id.clone(),
        worst_case_id: worst.case_id.clone(),
    }
}

fn sample_keys(parameters: &BTreeMap<String, ParameterValue>) -> Vec<String> {
    parameters.keys().cloned().collect()
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn validate_identifier(value: &str, field: &'static str) -> Result<(), SensitivityError> {
    if value.trim().is_empty() {
        return Err(SensitivityError::EmptyField(field));
    }
    Ok(())
}

#[derive(Debug, Error, PartialEq)]
pub enum SensitivityError {
    #[error("{0} cannot be empty")]
    EmptyField(&'static str),
    #[error("sweep definition must include at least one parameter")]
    EmptySweepParameters,
    #[error("parameter {0} must include at least one candidate value")]
    EmptyParameterValues(String),
    #[error("duplicate parameter name: {0}")]
    DuplicateParameter(String),
    #[error("invalid float parameter value: {0}")]
    InvalidFloatValue(String),
    #[error("epsilon must be finite and >= 0, got {0}")]
    InvalidEpsilon(f64),
    #[error("cannot analyze an empty sample set")]
    EmptySamples,
    #[error("sample {case_id} has a non-finite objective value: {objective}")]
    NonFiniteObjective { case_id: String, objective: f64 },
    #[error("sample {case_id} does not match the expected parameter keys")]
    InconsistentSampleParameters { case_id: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value_map(entries: &[(&str, ParameterValue)]) -> BTreeMap<String, ParameterValue> {
        entries
            .iter()
            .map(|(key, value)| ((*key).to_string(), value.clone()))
            .collect()
    }

    #[test]
    fn test_generate_grid_sweep_cases() {
        let mut definition = SweepDefinition::new("sweep", "Grid Sweep");
        definition.parameters.push(SweepParameter {
            name: "dose".into(),
            values: vec![ParameterValue::Int(1), ParameterValue::Int(2)],
        });
        definition.parameters.push(SweepParameter {
            name: "protocol".into(),
            values: vec![
                ParameterValue::Text("a".into()),
                ParameterValue::Text("b".into()),
            ],
        });

        let cases = generate_sweep_cases(&definition).unwrap();
        assert_eq!(cases.len(), 4);
        assert_eq!(cases[0].case_id, "sweep-1");
        assert!(cases
            .iter()
            .all(|case| case.parameters.contains_key("dose")
                && case.parameters.contains_key("protocol")));
    }

    #[test]
    fn test_analyze_numeric_sensitivity_ranks_strong_parameter_first() {
        let samples = vec![
            SweepSample {
                case_id: "c1".into(),
                parameters: value_map(&[
                    ("dose", ParameterValue::Int(1)),
                    ("variant", ParameterValue::Text("x".into())),
                ]),
                objective: 1.0,
                metadata: None,
            },
            SweepSample {
                case_id: "c2".into(),
                parameters: value_map(&[
                    ("dose", ParameterValue::Int(2)),
                    ("variant", ParameterValue::Text("x".into())),
                ]),
                objective: 2.0,
                metadata: None,
            },
            SweepSample {
                case_id: "c3".into(),
                parameters: value_map(&[
                    ("dose", ParameterValue::Int(3)),
                    ("variant", ParameterValue::Text("y".into())),
                ]),
                objective: 3.0,
                metadata: None,
            },
        ];

        let report = analyze_sensitivity(&samples, &SensitivityConfig::default()).unwrap();
        assert_eq!(report.rankings[0].parameter, "dose");
        assert!(matches!(
            report.rankings[0].kind,
            SensitivityKind::Numeric(_)
        ));
        assert_eq!(report.objective.best_case_id, "c3");
    }

    #[test]
    fn test_analyze_categorical_sensitivity() {
        let samples = vec![
            SweepSample {
                case_id: "c1".into(),
                parameters: value_map(&[("policy", ParameterValue::Text("hold".into()))]),
                objective: 0.5,
                metadata: None,
            },
            SweepSample {
                case_id: "c2".into(),
                parameters: value_map(&[("policy", ParameterValue::Text("hold".into()))]),
                objective: 0.6,
                metadata: None,
            },
            SweepSample {
                case_id: "c3".into(),
                parameters: value_map(&[("policy", ParameterValue::Text("boost".into()))]),
                objective: 0.9,
                metadata: None,
            },
        ];

        let report = analyze_sensitivity(&samples, &SensitivityConfig::default()).unwrap();
        match &report.rankings[0].kind {
            SensitivityKind::Categorical(stats) => {
                assert_eq!(stats.buckets.len(), 2);
                assert!(stats.range > 0.0);
            }
            _ => panic!("expected categorical stats"),
        }
    }

    #[test]
    fn test_analyze_sensitivity_rejects_inconsistent_parameters() {
        let samples = vec![
            SweepSample {
                case_id: "c1".into(),
                parameters: value_map(&[("dose", ParameterValue::Int(1))]),
                objective: 1.0,
                metadata: None,
            },
            SweepSample {
                case_id: "c2".into(),
                parameters: value_map(&[("protocol", ParameterValue::Text("a".into()))]),
                objective: 1.5,
                metadata: None,
            },
        ];

        let err = analyze_sensitivity(&samples, &SensitivityConfig::default()).unwrap_err();
        assert_eq!(
            err,
            SensitivityError::InconsistentSampleParameters {
                case_id: "c2".into()
            }
        );
    }
}
