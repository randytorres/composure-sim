//! Output mode builder — assembles the final MarketSimulationResult artifact.

use crate::schemas::{
    config_digest, BuyerOutcome, CohortOutcome, MarketSimulationConfig, MarketSimulationResult,
    MarketTotals,
};

/// Build the final output artifact from simulation components.
pub fn build_result(
    buyers: Vec<BuyerOutcome>,
    cohorts: Vec<CohortOutcome>,
    market_totals: MarketTotals,
    config: &MarketSimulationConfig,
    variant_count: usize,
    time_steps: usize,
) -> MarketSimulationResult {
    let digest = config_digest(config);
    MarketSimulationResult {
        buyers,
        cohorts,
        market_totals,
        config_digest: digest,
        variant_count,
        time_steps,
    }
}
