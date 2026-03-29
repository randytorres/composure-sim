//! Python bindings for composure-core.
//!
//! Exposes Monte Carlo engine, Composure Curve analysis, and core types to Python.
#![allow(clippy::useless_conversion)]

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use composure_core::composure;
use composure_core::monte_carlo::{self as mc, MonteCarloConfig};
use composure_core::state::SimState;
use composure_core::Simulator;

struct DefaultSimulator;

fn build_state(
    initial_z: Vec<f64>,
    initial_m: Vec<f64>,
    initial_u: Vec<f64>,
) -> PyResult<SimState> {
    SimState::try_new(initial_z, initial_m, initial_u)
        .map_err(|err| PyValueError::new_err(err.to_string()))
}

impl Simulator for DefaultSimulator {
    fn step(
        &self,
        state: &SimState,
        action: &composure_core::Action,
        rng: &mut dyn rand::RngCore,
    ) -> SimState {
        use rand::Rng;
        let mut next = state.clone();
        next.t += 1;
        for i in 0..next.z.len() {
            let noise = (rng.gen::<f64>() - 0.5) * 0.02;
            let memory_drag = next.m[i] * 0.01;
            let dim_magnitude = if action.dimension == Some(i) {
                action.magnitude
            } else {
                0.0
            };
            next.z[i] = (next.z[i] + dim_magnitude * 0.01 + noise - memory_drag).clamp(0.0, 1.0);
            next.m[i] = (next.m[i] * 0.95 + (1.0 - next.z[i]) * 0.05).clamp(0.0, 1.0);
            next.u[i] = (next.u[i] * 0.99).clamp(0.0, 1.0);
        }
        next
    }
}

/// Run a Monte Carlo simulation and return results as a Python dict.
#[pyfunction]
#[pyo3(signature = (initial_z, initial_m, initial_u, num_paths=10000, time_steps=180, seed=42))]
fn run_monte_carlo<'py>(
    py: Python<'py>,
    initial_z: Vec<f64>,
    initial_m: Vec<f64>,
    initial_u: Vec<f64>,
    num_paths: usize,
    time_steps: usize,
    seed: u64,
) -> PyResult<Bound<'py, PyDict>> {
    let state = build_state(initial_z, initial_m, initial_u)?;
    let config = MonteCarloConfig::with_seed(num_paths, time_steps, seed);
    let sim = DefaultSimulator;

    let result = mc::run_monte_carlo_checked(&sim, &state, &[], &config, false)
        .map_err(|err| PyValueError::new_err(err.to_string()))?;

    let dict = PyDict::new_bound(py);
    dict.set_item("mean_trajectory", &result.mean_trajectory)?;

    let percentiles = PyDict::new_bound(py);
    percentiles.set_item("p10", &result.percentiles.p10)?;
    percentiles.set_item("p25", &result.percentiles.p25)?;
    percentiles.set_item("p50", &result.percentiles.p50)?;
    percentiles.set_item("p75", &result.percentiles.p75)?;
    percentiles.set_item("p90", &result.percentiles.p90)?;
    dict.set_item("percentiles", percentiles)?;

    dict.set_item("num_paths", num_paths)?;
    dict.set_item("time_steps", time_steps)?;
    dict.set_item("seed", seed)?;

    Ok(dict)
}

/// Analyze a sequence of values into a Composure Curve.
#[pyfunction]
#[pyo3(signature = (values, threshold=0.3))]
fn analyze_composure<'py>(
    py: Python<'py>,
    values: Vec<f64>,
    threshold: f64,
) -> PyResult<Bound<'py, PyDict>> {
    let result = composure::analyze_composure_checked(&values, threshold)
        .map_err(|err| PyValueError::new_err(err.to_string()))?;

    let dict = PyDict::new_bound(py);
    dict.set_item("archetype", result.archetype.label())?;
    dict.set_item("archetype_description", result.archetype.description())?;

    let metrics = PyDict::new_bound(py);
    metrics.set_item("slope", result.metrics.slope)?;
    metrics.set_item("variance", result.metrics.variance)?;
    metrics.set_item("peak", result.metrics.peak)?;
    metrics.set_item("trough", result.metrics.trough)?;
    metrics.set_item("residual_damage", result.metrics.residual_damage)?;
    metrics.set_item("recovery_half_life", result.metrics.recovery_half_life)?;
    metrics.set_item("break_point", result.metrics.break_point)?;
    dict.set_item("metrics", metrics)?;

    let timeline = PyList::empty_bound(py);
    for point in &result.timeline {
        let p = PyDict::new_bound(py);
        p.set_item("t", point.t)?;
        p.set_item("value", point.value)?;
        timeline.append(p)?;
    }
    dict.set_item("timeline", timeline)?;

    Ok(dict)
}

/// Classify archetype from a sequence of values.
#[pyfunction]
fn classify_archetype(values: Vec<f64>) -> String {
    composure::classify_archetype(&values).label().to_string()
}

/// Compute linear regression slope.
#[pyfunction]
fn linear_slope(values: Vec<f64>) -> f64 {
    composure::linear_slope(&values)
}

/// Compute population variance.
#[pyfunction]
fn variance(values: Vec<f64>) -> f64 {
    composure::compute_variance(&values)
}

#[pymodule]
fn composure_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(run_monte_carlo, m)?)?;
    m.add_function(wrap_pyfunction!(analyze_composure, m)?)?;
    m.add_function(wrap_pyfunction!(classify_archetype, m)?)?;
    m.add_function(wrap_pyfunction!(linear_slope, m)?)?;
    m.add_function(wrap_pyfunction!(variance, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_simulator() {
        let sim = DefaultSimulator;
        let state = SimState::new(vec![0.5], vec![0.0], vec![0.5]);
        let action = composure_core::Action::default();
        let mut rng = rand::thread_rng();
        let next = sim.step(&state, &action, &mut rng);
        assert_eq!(next.t, 1);
    }
}
