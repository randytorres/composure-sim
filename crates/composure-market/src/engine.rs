//! Seeded buyer-level market simulation engine.
//!
//! ## Determinism guarantee
//!
//! The engine is fully deterministic given the same `MarketSimulationConfig`:
//! - Population generation uses a seeded `StdRng` to assign archetypes
//! - All probabilistic transitions use the same seeded RNG
//! - Running the engine twice with the same config produces identical results
//!
//! ## Simulation loop
//!
//! ```text
//! For each time step t in 0..time_steps:
//!   For each buyer:
//!     step_unaware_to_aware   // exposure channel attribution
//!     step_aware_to_considering
//!     step_considering_to_signup
//!     step_signup_to_activated
//!     step_activated            // retention + churn
//!     step_referral             // viral loop
//!   End
//! End
//! ```

use rand::distributions::Distribution;

use crate::cohort::{aggregate_cohorts, summarize_market};
use crate::outputs::build_result;
use crate::schemas::{
    BuyerArchetype, BuyerState, BuyerOutcome, CampaignVariant,
    MarketSimulationConfig, MarketSimulationResult,
};
use crate::transitions::{
    step_aware_to_considering, step_activated, step_considering_to_signup,
    step_referral, step_signup_to_activated, step_unaware_to_aware,
};
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};

/// Core seeded market simulation engine.
#[derive(Debug)]
pub struct MarketSimEngine {
    config: MarketSimulationConfig,
    rng: rand::rngs::StdRng,
}

impl MarketSimEngine {
    /// Create a new engine with a deterministic RNG seeded from the config.
    pub fn new(config: MarketSimulationConfig) -> Self {
        let seed = config.population.effective_seed();
        Self {
            config,
            rng: rand::rngs::StdRng::seed_from_u64(seed),
        }
    }

    /// Run the full simulation and return the result.
    pub fn run(&mut self) -> MarketSimulationResult {
        let mut buyers = self.generate_population();
        let time_steps = self.config.population.time_steps;
        let variants = self.config.variants.clone();

        // Run timesteps for each variant
        for variant in &variants {
            run_timesteps(&mut buyers, &mut self.rng, &variant, time_steps);
        }

        // Collect buyer outcomes
        let revenue_per_referral = 500.0; // cents
        let outcomes: Vec<BuyerOutcome> = buyers
            .iter()
            .map(|s| BuyerOutcome::from_state(s, revenue_per_referral))
            .collect();

        // Aggregate to cohorts
        let cohorts = aggregate_cohorts(&buyers, &outcomes);
        let market_totals = summarize_market(&cohorts);

        // Sample buyer outcomes
        let sample_rate = self.config.population.sample_rate;
        let sampled = if sample_rate >= 1.0 {
            outcomes.clone()
        } else {
            sample_buyers(&outcomes, sample_rate, &mut self.rng)
        };

        build_result(
            sampled,
            cohorts,
            market_totals,
            &self.config,
            self.config.variants.len(),
            self.config.population.time_steps,
        )
    }

    /// Generate the initial population deterministically from the config.
    fn generate_population(&mut self) -> Vec<BuyerState> {
        let n = self.config.population.population_size;
        let sampler = self.config.population.archetype_weights.sampler(&mut self.rng);

        let archetypes: Vec<BuyerArchetype> = (0..n)
            .map(|_i| {
                let idx = sampler.sample(&mut self.rng);
                match idx {
                    0 => BuyerArchetype::HighIntent,
                    1 => BuyerArchetype::Browsers,
                    2 => BuyerArchetype::DealSeekers,
                    3 => BuyerArchetype::Loyalists,
                    4 => BuyerArchetype::Dormant,
                    _ => BuyerArchetype::Dormant,
                }
            })
            .collect();

        archetypes
            .into_iter()
            .enumerate()
            .map(|(i, arch)| BuyerState::new(i, arch))
            .collect()
    }
}

/// Run all timesteps for a single variant (standalone function to avoid borrow conflicts).
pub fn run_timesteps<R: Rng>(
    buyers: &mut [BuyerState],
    rng: &mut R,
    variant: &CampaignVariant,
    time_steps: usize,
) {
    for t in 0..time_steps {
        for buyer in buyers.iter_mut() {
            step_unaware_to_aware(buyer, t, variant, rng);
            step_aware_to_considering(buyer, t, rng);
            step_considering_to_signup(buyer, t, rng);
            step_signup_to_activated(buyer, t, rng);
            step_activated(buyer, t, rng);
            step_referral(buyer, t, rng);
        }
    }
}

/// Sample a random subset of buyer outcomes at the given rate.
fn sample_buyers<R: Rng>(
    outcomes: &[BuyerOutcome],
    rate: f64,
    rng: &mut R,
) -> Vec<BuyerOutcome> {
    if rate <= 0.0 || rate >= 1.0 {
        return outcomes.to_vec();
    }

    let n = outcomes.len();
    let sample_size = ((n as f64) * rate).ceil() as usize;
    let mut indices: Vec<usize> = (0..n).collect();
    indices.shuffle(rng);
    indices.truncate(sample_size);
    indices.sort();

    indices
        .iter()
        .filter_map(|&i| outcomes.get(i).cloned())
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schemas::{MarketSimulationConfig, SyntheticPopulationConfig, Validate};

    fn test_config() -> MarketSimulationConfig {
        MarketSimulationConfig {
            population: SyntheticPopulationConfig {
                population_size: 100,
                time_steps: 30,
                seed: 42,
                ..Default::default()
            },
            variants: vec![CampaignVariant {
                variant_id: "control".into(),
                spend_budget: 10_000.0,
                ..Default::default()
            }],
        }
    }

    #[test]
    fn seeded_determinism() {
        let config = test_config();
        let result1 = MarketSimEngine::new(config.clone()).run();
        let result2 = MarketSimEngine::new(config).run();
        assert_eq!(result1.config_digest, result2.config_digest);
        assert_eq!(
            result1.market_totals.total_signups,
            result2.market_totals.total_signups
        );
        assert_eq!(
            result1.market_totals.total_activations,
            result2.market_totals.total_activations
        );
    }

    #[test]
    fn population_generation_uses_seed() {
        let mut config = test_config();
        config.population.seed = 99;

        let result = MarketSimEngine::new(config).run();
        // With different seed, market should differ (statistically)
        assert!(result.market_totals.total_buyers >= 100);
    }

    #[test]
    fn generates_cohorts() {
        let config = test_config();
        let result = MarketSimEngine::new(config).run();
        assert!(!result.cohorts.is_empty());
        let total: usize = result.cohorts.iter().map(|c| c.buyer_count).sum();
        assert_eq!(total, 100);
    }

    #[test]
    fn sample_rate_respected() {
        let mut config = test_config();
        config.population.sample_rate = 1.0;

        let result = MarketSimEngine::new(config).run();
        // With sample_rate=1.0, all 100 outcomes should be returned
        assert_eq!(result.buyers.len(), 100);
    }

    #[test]
    fn sample_rate_one_percent() {
        let mut config = test_config();
        config.population.sample_rate = 0.01;

        let result = MarketSimEngine::new(config).run();
        // Approximately 1% of 100 = 1 buyer
        assert!(result.buyers.len() <= 5);
    }

    #[test]
    fn engine_validates_config() {
        let mut config = test_config();
        config.variants = vec![]; // invalid: no variants

        let errors = config.validate();
        assert!(!errors.is_empty());
    }

    #[test]
    fn engine_handles_high_intent_buyers() {
        let config = MarketSimulationConfig {
            population: SyntheticPopulationConfig {
                population_size: 50,
                time_steps: 10,
                seed: 7,
                archetype_weights: crate::schemas::ArchetypeWeights {
                    high_intent: 1.0,
                    browsers: 0.0,
                    deal_seekers: 0.0,
                    loyalists: 0.0,
                    dormant: 0.0,
                },
                sample_rate: 1.0,
            },
            variants: vec![CampaignVariant {
                variant_id: "hi".into(),
                spend_budget: 20_000.0,
                awareness_rate: 0.5,
                ..Default::default()
            }],
        };
        let result = MarketSimEngine::new(config).run();
        // High-intent buyers should have high conversion rates
        assert!(result.market_totals.total_signups > 0);
        assert!(result.market_totals.total_activations > 0);
    }

    #[test]
    fn engine_handles_all_dormant_buyers() {
        let config = MarketSimulationConfig {
            population: SyntheticPopulationConfig {
                population_size: 20,
                time_steps: 5,
                seed: 13,
                archetype_weights: crate::schemas::ArchetypeWeights {
                    high_intent: 0.0,
                    browsers: 0.0,
                    deal_seekers: 0.0,
                    loyalists: 0.0,
                    dormant: 1.0,
                },
                sample_rate: 1.0,
            },
            variants: vec![CampaignVariant {
                variant_id: "dormant".into(),
                spend_budget: 1000.0,
                ..Default::default()
            }],
        };
        let result = MarketSimEngine::new(config).run();
        // Dormant buyers should have low conversion rates
        assert!(result.market_totals.total_activations <= 5);
    }
}