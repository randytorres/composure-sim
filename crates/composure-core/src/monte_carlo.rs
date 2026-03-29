use rand::rngs::StdRng;
use rand::SeedableRng;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::simulator::Simulator;
use crate::state::{Action, SimState};

/// Configuration for a Monte Carlo simulation run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonteCarloConfig {
    /// Number of parallel simulation paths.
    pub num_paths: usize,
    /// Number of time steps per path.
    pub time_steps: usize,
    /// Base seed for deterministic reproduction. Each path gets `seed_base + path_index`.
    pub seed_base: u64,
}

impl MonteCarloConfig {
    pub fn new(num_paths: usize, time_steps: usize) -> Self {
        Self {
            num_paths,
            time_steps,
            seed_base: 42,
        }
    }

    pub fn with_seed(num_paths: usize, time_steps: usize, seed: u64) -> Self {
        Self {
            num_paths,
            time_steps,
            seed_base: seed,
        }
    }

    pub fn validate(&self) -> Result<(), MonteCarloError> {
        if self.num_paths == 0 {
            return Err(MonteCarloError::ZeroPaths);
        }
        Ok(())
    }
}

/// Result of a single simulation path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathResult {
    /// Scalar health/performance index at each time step.
    pub health_indices: Vec<f64>,
    /// Final state at end of path.
    pub final_state: SimState,
    /// Seed used for this path (for replay).
    pub seed: u64,
}

/// Aggregate result of a Monte Carlo simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonteCarloResult {
    /// Individual path results (if `retain_paths` was true).
    pub paths: Vec<PathResult>,

    /// Percentile bands at each time step.
    pub percentiles: PercentileBands,

    /// Mean health index at each time step.
    pub mean_trajectory: Vec<f64>,

    /// Configuration used.
    pub config: MonteCarloConfig,
}

/// Percentile bands across all paths at each time step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PercentileBands {
    pub p10: Vec<f64>,
    pub p25: Vec<f64>,
    pub p50: Vec<f64>, // median
    pub p75: Vec<f64>,
    pub p90: Vec<f64>,
}

/// Run a Monte Carlo simulation with `rayon` parallelism.
///
/// Each path runs the `Simulator::step()` function `time_steps` times,
/// collecting the `health_index()` at each step. Paths are seeded
/// deterministically: path `i` uses seed `seed_base + i`.
///
/// # Arguments
/// - `sim`: Domain-specific simulator (implements `Simulator` trait).
/// - `initial_state`: Starting state for all paths.
/// - `actions`: Actions to apply at each time step. If shorter than `time_steps`,
///   remaining steps use `Action::default()` (Hold).
/// - `config`: Monte Carlo parameters.
/// - `retain_paths`: If true, keep all individual path results (memory-heavy for large runs).
pub fn run_monte_carlo<S: Simulator>(
    sim: &S,
    initial_state: &SimState,
    actions: &[Action],
    config: &MonteCarloConfig,
    retain_paths: bool,
) -> MonteCarloResult {
    run_monte_carlo_checked(sim, initial_state, actions, config, retain_paths)
        .expect("invalid Monte Carlo configuration")
}

/// Checked variant of [`run_monte_carlo`] that returns configuration errors.
pub fn run_monte_carlo_checked<S: Simulator>(
    sim: &S,
    initial_state: &SimState,
    actions: &[Action],
    config: &MonteCarloConfig,
    retain_paths: bool,
) -> Result<MonteCarloResult, MonteCarloError> {
    config.validate()?;

    // Run all paths in parallel
    let path_results: Vec<PathResult> = (0..config.num_paths)
        .into_par_iter()
        .map(|path_idx| {
            let seed = config.seed_base.wrapping_add(path_idx as u64);
            let mut rng = StdRng::seed_from_u64(seed);
            let mut state = initial_state.clone();
            let mut health_indices = Vec::with_capacity(config.time_steps);

            for t in 0..config.time_steps {
                let action = actions.get(t).cloned().unwrap_or_default();
                state = sim.step(&state, &action, &mut rng);
                health_indices.push(sim.health_index(&state));
            }

            PathResult {
                health_indices,
                final_state: state,
                seed,
            }
        })
        .collect();

    // Compute statistics across paths at each time step
    let time_steps = config.time_steps;
    let num_paths = path_results.len();

    let mut mean_trajectory = vec![0.0; time_steps];
    let mut columns: Vec<Vec<f64>> = vec![Vec::with_capacity(num_paths); time_steps];

    for path in &path_results {
        for (t, &val) in path.health_indices.iter().enumerate() {
            mean_trajectory[t] += val;
            columns[t].push(val);
        }
    }

    for t in 0..time_steps {
        mean_trajectory[t] /= num_paths as f64;
        columns[t].sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    }

    let percentiles = PercentileBands {
        p10: columns.iter().map(|c| percentile(c, 0.10)).collect(),
        p25: columns.iter().map(|c| percentile(c, 0.25)).collect(),
        p50: columns.iter().map(|c| percentile(c, 0.50)).collect(),
        p75: columns.iter().map(|c| percentile(c, 0.75)).collect(),
        p90: columns.iter().map(|c| percentile(c, 0.90)).collect(),
    };

    Ok(MonteCarloResult {
        paths: if retain_paths { path_results } else { vec![] },
        percentiles,
        mean_trajectory,
        config: config.clone(),
    })
}

/// Compute the p-th percentile from a sorted slice.
fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = (p * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum MonteCarloError {
    #[error("num_paths must be greater than zero")]
    ZeroPaths,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::ActionType;

    /// Trivial simulator: z drifts toward action magnitude with noise.
    struct DriftSim;

    impl Simulator for DriftSim {
        fn step(&self, state: &SimState, action: &Action, rng: &mut dyn rand::RngCore) -> SimState {
            use rand::Rng;
            let mut next = state.clone();
            next.t += 1;
            for i in 0..next.z.len() {
                let noise = (rng.gen::<f64>() - 0.5) * 0.05;
                next.z[i] = (next.z[i] + action.magnitude * 0.01 + noise).clamp(0.0, 1.0);
            }
            next
        }
    }

    #[test]
    fn test_monte_carlo_runs() {
        let sim = DriftSim;
        let initial = SimState::new(vec![0.5], vec![0.0], vec![0.5]);
        let actions = vec![Action {
            dimension: Some(0),
            magnitude: 1.0,
            action_type: ActionType::Intervention,
            metadata: None,
        }];
        let config = MonteCarloConfig::with_seed(100, 30, 42);

        let result = run_monte_carlo(&sim, &initial, &actions, &config, false);

        assert_eq!(result.mean_trajectory.len(), 30);
        assert_eq!(result.percentiles.p50.len(), 30);
        assert!(result.paths.is_empty()); // retain_paths = false
    }

    #[test]
    fn test_deterministic() {
        let sim = DriftSim;
        let initial = SimState::new(vec![0.5], vec![0.0], vec![0.5]);
        let config = MonteCarloConfig::with_seed(50, 20, 123);

        let r1 = run_monte_carlo(&sim, &initial, &[], &config, false);
        let r2 = run_monte_carlo(&sim, &initial, &[], &config, false);

        assert_eq!(r1.mean_trajectory, r2.mean_trajectory);
    }

    #[test]
    fn test_retain_paths() {
        let sim = DriftSim;
        let initial = SimState::new(vec![0.5], vec![0.0], vec![0.5]);
        let config = MonteCarloConfig::with_seed(10, 5, 0);

        let result = run_monte_carlo(&sim, &initial, &[], &config, true);

        assert_eq!(result.paths.len(), 10);
        assert_eq!(result.paths[0].health_indices.len(), 5);
    }

    #[test]
    fn test_zero_paths_rejected() {
        let sim = DriftSim;
        let initial = SimState::new(vec![0.5], vec![0.0], vec![0.5]);
        let config = MonteCarloConfig::with_seed(0, 5, 0);

        let err = run_monte_carlo_checked(&sim, &initial, &[], &config, false).unwrap_err();
        assert_eq!(err, MonteCarloError::ZeroPaths);
    }

    #[test]
    fn test_percentile_ordering() {
        let sim = DriftSim;
        let initial = SimState::new(vec![0.5], vec![0.0], vec![0.5]);
        let config = MonteCarloConfig::with_seed(1000, 10, 42);

        let result = run_monte_carlo(&sim, &initial, &[], &config, false);

        for t in 0..10 {
            assert!(result.percentiles.p10[t] <= result.percentiles.p25[t]);
            assert!(result.percentiles.p25[t] <= result.percentiles.p50[t]);
            assert!(result.percentiles.p50[t] <= result.percentiles.p75[t]);
            assert!(result.percentiles.p75[t] <= result.percentiles.p90[t]);
        }
    }
}
