use serde::{Deserialize, Serialize};

use crate::{Archetype, RunSummary, TrajectoryComparison};

/// Generic scalar delta between a baseline and candidate summary field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryDelta {
    pub baseline: Option<f64>,
    pub candidate: Option<f64>,
    pub delta: Option<f64>,
}

/// Change in composure archetype classification between two runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchetypeChange {
    pub baseline: Option<Archetype>,
    pub candidate: Option<Archetype>,
    pub changed: bool,
}

/// Shift in break-point timing between two composure summaries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakPointShift {
    pub baseline: Option<usize>,
    pub candidate: Option<usize>,
    /// Positive means the candidate breaks later than the baseline.
    pub shift: Option<isize>,
}

/// Shift in recovery half-life between two composure summaries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryShift {
    pub baseline: Option<usize>,
    pub candidate: Option<usize>,
    /// Negative means the candidate recovers faster.
    pub shift: Option<isize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BandChangeDirection {
    Widened,
    Narrowed,
    Unchanged,
    Unknown,
}

/// Final-percentile-band change between two Monte Carlo summaries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PercentileBandChange {
    pub baseline: Option<f64>,
    pub candidate: Option<f64>,
    pub delta: Option<f64>,
    pub direction: BandChangeDirection,
}

/// Selected trajectory-comparison metrics copied into a report-friendly shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonSnapshot {
    pub mean_delta: f64,
    pub mean_abs_delta: f64,
    pub rmse: f64,
    pub end_delta: f64,
    pub divergence_start_t: Option<usize>,
    pub divergence_end_t: Option<usize>,
    pub failure_shift: Option<isize>,
}

/// Compact deterministic report artifact comparing two runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeterministicReport {
    pub start_delta: SummaryDelta,
    pub end_delta: SummaryDelta,
    pub auc_delta: SummaryDelta,
    pub residual_damage_delta: SummaryDelta,
    pub archetype_change: ArchetypeChange,
    pub break_point_shift: BreakPointShift,
    pub recovery_shift: RecoveryShift,
    pub percentile_band_change: PercentileBandChange,
    pub comparison: Option<ComparisonSnapshot>,
}

/// Build a deterministic comparison report from compact run summaries and an optional trajectory comparison.
pub fn build_deterministic_report(
    baseline: &RunSummary,
    candidate: &RunSummary,
    comparison: Option<&TrajectoryComparison>,
) -> DeterministicReport {
    let baseline_monte_carlo = baseline.monte_carlo.as_ref();
    let candidate_monte_carlo = candidate.monte_carlo.as_ref();
    let baseline_composure = baseline.composure.as_ref();
    let candidate_composure = candidate.composure.as_ref();

    DeterministicReport {
        start_delta: scalar_delta(
            baseline_monte_carlo.and_then(|summary| summary.start),
            candidate_monte_carlo.and_then(|summary| summary.start),
        ),
        end_delta: scalar_delta(
            baseline_monte_carlo.and_then(|summary| summary.end),
            candidate_monte_carlo.and_then(|summary| summary.end),
        ),
        auc_delta: scalar_delta(
            baseline_monte_carlo.and_then(|summary| summary.auc),
            candidate_monte_carlo.and_then(|summary| summary.auc),
        ),
        residual_damage_delta: scalar_delta(
            baseline_composure.map(|summary| summary.residual_damage),
            candidate_composure.map(|summary| summary.residual_damage),
        ),
        archetype_change: ArchetypeChange {
            baseline: baseline_composure.map(|summary| summary.archetype),
            candidate: candidate_composure.map(|summary| summary.archetype),
            changed: match (baseline_composure, candidate_composure) {
                (Some(baseline), Some(candidate)) => baseline.archetype != candidate.archetype,
                _ => false,
            },
        },
        break_point_shift: ordinal_shift(
            baseline_composure.and_then(|summary| summary.break_point),
            candidate_composure.and_then(|summary| summary.break_point),
        ),
        recovery_shift: recovery_shift(
            baseline_composure.and_then(|summary| summary.recovery_half_life),
            candidate_composure.and_then(|summary| summary.recovery_half_life),
        ),
        percentile_band_change: band_change(
            baseline_monte_carlo.and_then(|summary| summary.final_band_width),
            candidate_monte_carlo.and_then(|summary| summary.final_band_width),
        ),
        comparison: comparison.map(|comparison| ComparisonSnapshot {
            mean_delta: comparison.metrics.mean_delta,
            mean_abs_delta: comparison.metrics.mean_abs_delta,
            rmse: comparison.metrics.rmse,
            end_delta: comparison.metrics.end_delta,
            divergence_start_t: comparison.divergence.as_ref().map(|window| window.start_t),
            divergence_end_t: comparison.divergence.as_ref().map(|window| window.end_t),
            failure_shift: comparison
                .metrics
                .failure
                .as_ref()
                .and_then(|failure| failure.shift),
        }),
    }
}

fn scalar_delta(baseline: Option<f64>, candidate: Option<f64>) -> SummaryDelta {
    SummaryDelta {
        baseline,
        candidate,
        delta: baseline
            .zip(candidate)
            .map(|(baseline, candidate)| candidate - baseline),
    }
}

fn ordinal_shift(baseline: Option<usize>, candidate: Option<usize>) -> BreakPointShift {
    BreakPointShift {
        baseline,
        candidate,
        shift: baseline
            .zip(candidate)
            .map(|(baseline, candidate)| candidate as isize - baseline as isize),
    }
}

fn recovery_shift(baseline: Option<usize>, candidate: Option<usize>) -> RecoveryShift {
    RecoveryShift {
        baseline,
        candidate,
        shift: baseline
            .zip(candidate)
            .map(|(baseline, candidate)| candidate as isize - baseline as isize),
    }
}

fn band_change(baseline: Option<f64>, candidate: Option<f64>) -> PercentileBandChange {
    let delta = baseline
        .zip(candidate)
        .map(|(baseline, candidate)| candidate - baseline);
    let direction = match delta {
        Some(delta) if delta > 0.0 => BandChangeDirection::Widened,
        Some(delta) if delta < 0.0 => BandChangeDirection::Narrowed,
        Some(_) => BandChangeDirection::Unchanged,
        None => BandChangeDirection::Unknown,
    };

    PercentileBandChange {
        baseline,
        candidate,
        delta,
        direction,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        compare_trajectories, ComparisonConfig, ComposureSummary, MonteCarloSummary, RunSummary,
    };

    fn baseline_summary() -> RunSummary {
        RunSummary {
            monte_carlo: Some(MonteCarloSummary {
                time_steps: 4,
                num_paths: 10,
                start: Some(0.9),
                end: Some(0.5),
                min: Some(0.5),
                max: Some(0.9),
                mean: Some(0.7),
                auc: Some(2.3),
                p10_end: Some(0.4),
                p50_end: Some(0.5),
                p90_end: Some(0.6),
                final_band_width: Some(0.2),
            }),
            composure: Some(ComposureSummary {
                archetype: Archetype::CliffFaller,
                slope: -0.1,
                variance: 0.03,
                peak: 0.9,
                trough: 0.5,
                recovery_half_life: Some(3),
                residual_damage: 0.4,
                break_point: Some(2),
            }),
        }
    }

    fn candidate_summary() -> RunSummary {
        RunSummary {
            monte_carlo: Some(MonteCarloSummary {
                time_steps: 4,
                num_paths: 10,
                start: Some(0.9),
                end: Some(0.7),
                min: Some(0.7),
                max: Some(0.9),
                mean: Some(0.78),
                auc: Some(2.9),
                p10_end: Some(0.66),
                p50_end: Some(0.7),
                p90_end: Some(0.76),
                final_band_width: Some(0.1),
            }),
            composure: Some(ComposureSummary {
                archetype: Archetype::Phoenix,
                slope: -0.03,
                variance: 0.01,
                peak: 0.9,
                trough: 0.65,
                recovery_half_life: Some(1),
                residual_damage: 0.2,
                break_point: Some(3),
            }),
        }
    }

    #[test]
    fn test_build_deterministic_report_uses_summary_deltas() {
        let report = build_deterministic_report(&baseline_summary(), &candidate_summary(), None);

        assert!((report.end_delta.delta.unwrap() - 0.2).abs() < 1e-9);
        assert!((report.auc_delta.delta.unwrap() - 0.6).abs() < 1e-9);
        assert!((report.residual_damage_delta.delta.unwrap() + 0.2).abs() < 1e-9);
        assert!(report.archetype_change.changed);
        assert_eq!(report.break_point_shift.shift, Some(1));
        assert_eq!(report.recovery_shift.shift, Some(-2));
        assert_eq!(
            report.percentile_band_change.direction,
            BandChangeDirection::Narrowed
        );
    }

    #[test]
    fn test_build_deterministic_report_embeds_comparison_snapshot() {
        let comparison = compare_trajectories(
            &[0.9, 0.8, 0.5, 0.5],
            &[0.9, 0.85, 0.7, 0.7],
            &ComparisonConfig {
                divergence_threshold: 0.1,
                sustained_steps: 2,
                failure_threshold: Some(0.6),
                ..ComparisonConfig::default()
            },
        )
        .unwrap();

        let report = build_deterministic_report(
            &baseline_summary(),
            &candidate_summary(),
            Some(&comparison),
        );

        let snapshot = report.comparison.unwrap();
        assert!(snapshot.mean_delta > 0.0);
        assert_eq!(snapshot.divergence_start_t, Some(2));
        assert_eq!(snapshot.divergence_end_t, Some(3));
        assert_eq!(snapshot.failure_shift, None);
    }
}
