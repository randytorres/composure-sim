//! WASM bindings for composure-core.
//!
//! Enables client-side Monte Carlo simulation in the browser (React web app)
//! and Composure Curve analysis without server round-trips.
//!
//! Build with: `wasm-pack build --target web crates/composure-wasm`

use wasm_bindgen::prelude::*;

use composure_core::composure;
use composure_core::monte_carlo::{self as mc, MonteCarloConfig};
use composure_core::state::SimState;
use composure_core::Simulator;

/// Default browser-side simulator (same as Python version).
struct BrowserSimulator;

fn js_error(message: impl Into<String>) -> JsValue {
    JsValue::from_str(&message.into())
}

impl Simulator for BrowserSimulator {
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
            let dim_mag = if action.dimension == Some(i) {
                action.magnitude
            } else {
                0.0
            };
            next.z[i] = (next.z[i] + dim_mag * 0.01 + noise - memory_drag).clamp(0.0, 1.0);
            next.m[i] = (next.m[i] * 0.95 + (1.0 - next.z[i]) * 0.05).clamp(0.0, 1.0);
            next.u[i] = (next.u[i] * 0.99).clamp(0.0, 1.0);
        }
        next
    }
}

/// Run Monte Carlo simulation in the browser. Returns JSON string.
#[wasm_bindgen]
pub fn run_monte_carlo(
    initial_z: &[f64],
    initial_m: &[f64],
    initial_u: &[f64],
    num_paths: usize,
    time_steps: usize,
    seed: u64,
) -> Result<String, JsValue> {
    let state = SimState::try_new(initial_z.to_vec(), initial_m.to_vec(), initial_u.to_vec())
        .map_err(|err| js_error(err.to_string()))?;
    // Use fewer paths in browser for performance (caller controls this)
    let config = MonteCarloConfig::with_seed(num_paths, time_steps, seed);
    let sim = BrowserSimulator;

    let result = mc::run_monte_carlo_checked(&sim, &state, &[], &config, false)
        .map_err(|err| js_error(err.to_string()))?;

    serde_json::to_string(&result).map_err(|err| js_error(err.to_string()))
}

/// Analyze a composure curve from health index values. Returns JSON string.
#[wasm_bindgen]
pub fn analyze_composure(values: &[f64], threshold: f64) -> Result<String, JsValue> {
    let result = composure::analyze_composure_checked(values, threshold)
        .map_err(|err| js_error(err.to_string()))?;
    serde_json::to_string(&result).map_err(|err| js_error(err.to_string()))
}

/// Quick archetype classification.
#[wasm_bindgen]
pub fn classify_archetype(values: &[f64]) -> String {
    composure::classify_archetype(values).label().to_string()
}
