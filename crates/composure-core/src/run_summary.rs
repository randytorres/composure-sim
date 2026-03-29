use serde::{Deserialize, Serialize};

use crate::{Archetype, ComposureCurve, MonteCarloResult};

/// Deterministic scalar summary extracted from a Monte Carlo result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonteCarloSummary {
    pub time_steps: usize,
    pub num_paths: usize,
    pub start: Option<f64>,
    pub end: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub mean: Option<f64>,
    /// Trapezoidal area under the mean trajectory.
    pub auc: Option<f64>,
    pub p10_end: Option<f64>,
    pub p50_end: Option<f64>,
    pub p90_end: Option<f64>,
    pub final_band_width: Option<f64>,
}

/// Deterministic scalar summary extracted from a composure curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposureSummary {
    pub archetype: Archetype,
    pub slope: f64,
    pub variance: f64,
    pub peak: f64,
    pub trough: f64,
    pub recovery_half_life: Option<usize>,
    pub residual_damage: f64,
    pub break_point: Option<usize>,
}

/// Compact report-friendly bundle of deterministic run metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunSummary {
    pub monte_carlo: Option<MonteCarloSummary>,
    pub composure: Option<ComposureSummary>,
}

pub fn summarize_monte_carlo(result: &MonteCarloResult) -> MonteCarloSummary {
    let trajectory = &result.mean_trajectory;
    let start = trajectory.first().copied();
    let end = trajectory.last().copied();
    let min = trajectory
        .iter()
        .copied()
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let max = trajectory
        .iter()
        .copied()
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mean = if trajectory.is_empty() {
        None
    } else {
        Some(trajectory.iter().sum::<f64>() / trajectory.len() as f64)
    };
    let auc = if trajectory.len() < 2 {
        trajectory.first().copied()
    } else {
        Some(
            trajectory
                .windows(2)
                .map(|window| (window[0] + window[1]) * 0.5)
                .sum::<f64>(),
        )
    };

    let p10_end = result.percentiles.p10.last().copied();
    let p50_end = result.percentiles.p50.last().copied();
    let p90_end = result.percentiles.p90.last().copied();
    let final_band_width = match (p10_end, p90_end) {
        (Some(low), Some(high)) => Some(high - low),
        _ => None,
    };

    MonteCarloSummary {
        time_steps: result.config.time_steps,
        num_paths: result.config.num_paths,
        start,
        end,
        min,
        max,
        mean,
        auc,
        p10_end,
        p50_end,
        p90_end,
        final_band_width,
    }
}

pub fn summarize_composure(curve: &ComposureCurve) -> ComposureSummary {
    ComposureSummary {
        archetype: curve.archetype,
        slope: curve.metrics.slope,
        variance: curve.metrics.variance,
        peak: curve.metrics.peak,
        trough: curve.metrics.trough,
        recovery_half_life: curve.metrics.recovery_half_life,
        residual_damage: curve.metrics.residual_damage,
        break_point: curve.metrics.break_point,
    }
}

pub fn summarize_run(
    monte_carlo: Option<&MonteCarloResult>,
    composure: Option<&ComposureCurve>,
) -> RunSummary {
    RunSummary {
        monte_carlo: monte_carlo.map(summarize_monte_carlo),
        composure: composure.map(summarize_composure),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        analyze_composure_checked,
        monte_carlo::{MonteCarloConfig, PercentileBands},
    };

    fn sample_monte_carlo() -> MonteCarloResult {
        MonteCarloResult {
            paths: vec![],
            percentiles: PercentileBands {
                p10: vec![0.5, 0.4, 0.3],
                p25: vec![0.55, 0.45, 0.35],
                p50: vec![0.6, 0.5, 0.4],
                p75: vec![0.65, 0.55, 0.45],
                p90: vec![0.7, 0.6, 0.5],
            },
            mean_trajectory: vec![0.6, 0.5, 0.4],
            config: MonteCarloConfig::with_seed(100, 3, 42),
        }
    }

    #[test]
    fn test_summarize_monte_carlo() {
        let summary = summarize_monte_carlo(&sample_monte_carlo());

        assert_eq!(summary.time_steps, 3);
        assert_eq!(summary.num_paths, 100);
        assert_eq!(summary.start, Some(0.6));
        assert_eq!(summary.end, Some(0.4));
        assert_eq!(summary.min, Some(0.4));
        assert_eq!(summary.max, Some(0.6));
        assert_eq!(summary.p50_end, Some(0.4));
        assert_eq!(summary.final_band_width, Some(0.2));
        assert!(summary.auc.unwrap() > 0.0);
    }

    #[test]
    fn test_summarize_composure() {
        let curve = analyze_composure_checked(&[0.9, 0.8, 0.5, 0.7], 0.6).unwrap();
        let summary = summarize_composure(&curve);

        assert_eq!(summary.archetype, curve.archetype);
        assert_eq!(summary.break_point, curve.metrics.break_point);
        assert_eq!(summary.residual_damage, curve.metrics.residual_damage);
    }

    #[test]
    fn test_summarize_run_combines_outputs() {
        let monte_carlo = sample_monte_carlo();
        let curve = analyze_composure_checked(&monte_carlo.mean_trajectory, 0.45).unwrap();

        let summary = summarize_run(Some(&monte_carlo), Some(&curve));

        assert!(summary.monte_carlo.is_some());
        assert!(summary.composure.is_some());
    }
}
