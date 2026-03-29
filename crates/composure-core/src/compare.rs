use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::monte_carlo::MonteCarloResult;

/// Configuration for comparing two equal-length trajectories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonConfig {
    /// Minimum absolute delta required to count as a divergence candidate.
    pub divergence_threshold: f64,
    /// Number of consecutive divergent steps required to report divergence.
    pub sustained_steps: usize,
    /// Small tolerance used when classifying improvements vs regressions.
    pub equality_epsilon: f64,
    /// Optional threshold used to compare first failure/break times.
    pub failure_threshold: Option<f64>,
}

impl Default for ComparisonConfig {
    fn default() -> Self {
        Self {
            divergence_threshold: 0.1,
            sustained_steps: 1,
            equality_epsilon: 1e-9,
            failure_threshold: None,
        }
    }
}

impl ComparisonConfig {
    pub fn validate(&self) -> Result<(), CompareError> {
        if !self.divergence_threshold.is_finite() || self.divergence_threshold < 0.0 {
            return Err(CompareError::InvalidDivergenceThreshold(
                self.divergence_threshold,
            ));
        }
        if self.sustained_steps == 0 {
            return Err(CompareError::ZeroSustainedSteps);
        }
        if !self.equality_epsilon.is_finite() || self.equality_epsilon < 0.0 {
            return Err(CompareError::InvalidEqualityEpsilon(self.equality_epsilon));
        }
        if let Some(threshold) = self.failure_threshold {
            if !threshold.is_finite() {
                return Err(CompareError::InvalidFailureThreshold(threshold));
            }
        }
        Ok(())
    }
}

/// Per-step comparison data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointDelta {
    pub t: usize,
    pub baseline: f64,
    pub candidate: f64,
    pub delta: f64,
    pub abs_delta: f64,
}

/// Summary of the largest positive or negative shift.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointDeltaSummary {
    pub t: usize,
    pub baseline: f64,
    pub candidate: f64,
    pub delta: f64,
}

/// A sustained period where the trajectories materially separate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DivergenceWindow {
    pub start_t: usize,
    pub end_t: usize,
    pub length: usize,
    pub peak_abs_delta: f64,
}

/// Failure/break-point comparison when a threshold is supplied.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FailureComparison {
    pub threshold: f64,
    pub baseline_break_t: Option<usize>,
    pub candidate_break_t: Option<usize>,
    /// Positive means the candidate failed later than the baseline.
    pub shift: Option<isize>,
    pub outcome: FailureComparisonOutcome,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FailureComparisonOutcome {
    NeitherFailed,
    BaselineOnly,
    CandidateOnly,
    BothFailed,
}

/// Aggregate metrics for a comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonMetrics {
    pub cumulative_delta: f64,
    pub mean_delta: f64,
    pub mean_abs_delta: f64,
    pub rmse: f64,
    pub end_delta: f64,
    pub improved_steps: usize,
    pub regressed_steps: usize,
    pub unchanged_steps: usize,
    pub max_improvement: PointDeltaSummary,
    pub max_regression: PointDeltaSummary,
    pub failure: Option<FailureComparison>,
}

/// Serializable artifact representing a counterfactual comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryComparison {
    pub series_len: usize,
    pub deltas: Vec<PointDelta>,
    pub metrics: ComparisonMetrics,
    pub divergence: Option<DivergenceWindow>,
    pub config: ComparisonConfig,
}

/// Compare two same-length trajectories and produce reusable summary artifacts.
pub fn compare_trajectories(
    baseline: &[f64],
    candidate: &[f64],
    config: &ComparisonConfig,
) -> Result<TrajectoryComparison, CompareError> {
    config.validate()?;

    if baseline.is_empty() || candidate.is_empty() {
        return Err(CompareError::EmptyTrajectory);
    }
    if baseline.len() != candidate.len() {
        return Err(CompareError::MismatchedLengths {
            baseline_len: baseline.len(),
            candidate_len: candidate.len(),
        });
    }

    let deltas: Vec<PointDelta> = baseline
        .iter()
        .zip(candidate.iter())
        .enumerate()
        .map(|(t, (&base, &cand))| {
            let delta = cand - base;
            PointDelta {
                t,
                baseline: base,
                candidate: cand,
                delta,
                abs_delta: delta.abs(),
            }
        })
        .collect();

    let cumulative_delta = deltas.iter().map(|p| p.delta).sum::<f64>();
    let mean_delta = cumulative_delta / deltas.len() as f64;
    let mean_abs_delta = deltas.iter().map(|p| p.abs_delta).sum::<f64>() / deltas.len() as f64;
    let rmse = (deltas.iter().map(|p| p.delta.powi(2)).sum::<f64>() / deltas.len() as f64).sqrt();
    let end_delta = deltas.last().map(|p| p.delta).unwrap_or(0.0);

    let improved_steps = deltas
        .iter()
        .filter(|p| p.delta > config.equality_epsilon)
        .count();
    let regressed_steps = deltas
        .iter()
        .filter(|p| p.delta < -config.equality_epsilon)
        .count();
    let unchanged_steps = deltas.len() - improved_steps - regressed_steps;

    let max_improvement = deltas
        .iter()
        .max_by(|a, b| a.delta.partial_cmp(&b.delta).unwrap())
        .map(to_summary)
        .expect("deltas is non-empty");
    let max_regression = deltas
        .iter()
        .min_by(|a, b| a.delta.partial_cmp(&b.delta).unwrap())
        .map(to_summary)
        .expect("deltas is non-empty");

    let divergence = detect_divergence(&deltas, config);
    let failure = config
        .failure_threshold
        .map(|threshold| compare_failures(baseline, candidate, threshold));

    Ok(TrajectoryComparison {
        series_len: deltas.len(),
        deltas,
        metrics: ComparisonMetrics {
            cumulative_delta,
            mean_delta,
            mean_abs_delta,
            rmse,
            end_delta,
            improved_steps,
            regressed_steps,
            unchanged_steps,
            max_improvement,
            max_regression,
            failure,
        },
        divergence,
        config: config.clone(),
    })
}

/// Convenience helper for comparing Monte Carlo mean trajectories.
pub fn compare_monte_carlo_results(
    baseline: &MonteCarloResult,
    candidate: &MonteCarloResult,
    config: &ComparisonConfig,
) -> Result<TrajectoryComparison, CompareError> {
    compare_trajectories(
        &baseline.mean_trajectory,
        &candidate.mean_trajectory,
        config,
    )
}

fn to_summary(point: &PointDelta) -> PointDeltaSummary {
    PointDeltaSummary {
        t: point.t,
        baseline: point.baseline,
        candidate: point.candidate,
        delta: point.delta,
    }
}

fn detect_divergence(deltas: &[PointDelta], config: &ComparisonConfig) -> Option<DivergenceWindow> {
    let mut start = None;
    let mut peak_abs_delta: f64 = 0.0;

    for point in deltas {
        if point.abs_delta >= config.divergence_threshold {
            start.get_or_insert(point.t);
            peak_abs_delta = peak_abs_delta.max(point.abs_delta);
            continue;
        }

        if let Some(start_t) = start.take() {
            let end_t = point.t.saturating_sub(1);
            let length = end_t - start_t + 1;
            if length >= config.sustained_steps {
                return Some(DivergenceWindow {
                    start_t,
                    end_t,
                    length,
                    peak_abs_delta,
                });
            }
            peak_abs_delta = 0.0;
        }
    }

    start.map(|start_t| {
        let end_t = deltas.len() - 1;
        let length = end_t - start_t + 1;
        if length >= config.sustained_steps {
            Some(DivergenceWindow {
                start_t,
                end_t,
                length,
                peak_abs_delta,
            })
        } else {
            None
        }
    })?
}

fn compare_failures(baseline: &[f64], candidate: &[f64], threshold: f64) -> FailureComparison {
    let baseline_break_t = baseline.iter().position(|&value| value < threshold);
    let candidate_break_t = candidate.iter().position(|&value| value < threshold);

    let (outcome, shift) = match (baseline_break_t, candidate_break_t) {
        (None, None) => (FailureComparisonOutcome::NeitherFailed, None),
        (Some(_), None) => (FailureComparisonOutcome::BaselineOnly, None),
        (None, Some(_)) => (FailureComparisonOutcome::CandidateOnly, None),
        (Some(base), Some(cand)) => (
            FailureComparisonOutcome::BothFailed,
            Some(cand as isize - base as isize),
        ),
    };

    FailureComparison {
        threshold,
        baseline_break_t,
        candidate_break_t,
        shift,
        outcome,
    }
}

#[derive(Debug, Clone, PartialEq, Error)]
pub enum CompareError {
    #[error("cannot compare empty trajectories")]
    EmptyTrajectory,
    #[error(
        "baseline and candidate must have the same length (got {baseline_len} and {candidate_len})"
    )]
    MismatchedLengths {
        baseline_len: usize,
        candidate_len: usize,
    },
    #[error("divergence_threshold must be finite and >= 0, got {0}")]
    InvalidDivergenceThreshold(f64),
    #[error("sustained_steps must be greater than zero")]
    ZeroSustainedSteps,
    #[error("equality_epsilon must be finite and >= 0, got {0}")]
    InvalidEqualityEpsilon(f64),
    #[error("failure_threshold must be finite, got {0}")]
    InvalidFailureThreshold(f64),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monte_carlo::{MonteCarloConfig, MonteCarloResult, PercentileBands};

    fn empty_percentiles() -> PercentileBands {
        PercentileBands {
            p10: vec![],
            p25: vec![],
            p50: vec![],
            p75: vec![],
            p90: vec![],
        }
    }

    #[test]
    fn test_compare_trajectories_rejects_mismatched_lengths() {
        let err =
            compare_trajectories(&[0.8, 0.7], &[0.8], &ComparisonConfig::default()).unwrap_err();

        assert_eq!(
            err,
            CompareError::MismatchedLengths {
                baseline_len: 2,
                candidate_len: 1,
            }
        );
    }

    #[test]
    fn test_compare_trajectories_detects_divergence() {
        let config = ComparisonConfig {
            divergence_threshold: 0.15,
            sustained_steps: 2,
            ..ComparisonConfig::default()
        };

        let result = compare_trajectories(
            &[0.9, 0.88, 0.86, 0.84, 0.82],
            &[0.9, 0.88, 0.64, 0.6, 0.58],
            &config,
        )
        .unwrap();

        let divergence = result.divergence.expect("expected divergence");
        assert_eq!(divergence.start_t, 2);
        assert_eq!(divergence.end_t, 4);
        assert_eq!(divergence.length, 3);
        assert!(divergence.peak_abs_delta >= 0.22);
    }

    #[test]
    fn test_compare_trajectories_reports_failure_shift() {
        let config = ComparisonConfig {
            failure_threshold: Some(0.5),
            ..ComparisonConfig::default()
        };

        let result = compare_trajectories(&[0.9, 0.45, 0.2], &[0.9, 0.7, 0.45], &config).unwrap();

        let failure = result.metrics.failure.expect("expected failure comparison");
        assert_eq!(failure.baseline_break_t, Some(1));
        assert_eq!(failure.candidate_break_t, Some(2));
        assert_eq!(failure.shift, Some(1));
        assert_eq!(failure.outcome, FailureComparisonOutcome::BothFailed);
    }

    #[test]
    fn test_compare_monte_carlo_results_uses_mean_trajectories() {
        let baseline = MonteCarloResult {
            paths: vec![],
            percentiles: empty_percentiles(),
            mean_trajectory: vec![0.9, 0.8, 0.7],
            config: MonteCarloConfig::with_seed(10, 3, 1),
        };
        let candidate = MonteCarloResult {
            paths: vec![],
            percentiles: empty_percentiles(),
            mean_trajectory: vec![0.9, 0.82, 0.75],
            config: MonteCarloConfig::with_seed(10, 3, 2),
        };

        let result =
            compare_monte_carlo_results(&baseline, &candidate, &ComparisonConfig::default())
                .unwrap();

        assert_eq!(result.series_len, 3);
        assert!(result.metrics.cumulative_delta > 0.0);
        assert_eq!(result.metrics.improved_steps, 2);
    }
}
