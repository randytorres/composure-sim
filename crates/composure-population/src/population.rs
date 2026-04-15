//! Population generator — Task 14.
//!
//! Expands a set of `SegmentBlueprint`s into N buyers (10k–100k) with stable IDs
//! and seeded reproducibility.

use crate::{blueprint::SegmentBlueprint, buyer::Buyer, traits::TraitDistribution};
use rand::{Rng, RngCore, SeedableRng};
use rand_chacha::ChaCha12Rng;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use std::collections::BTreeMap;

/// Stable buyer ID derived from population seed + index.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BuyerId(pub String);

impl BuyerId {
    pub fn new(population_id: &str, index: usize) -> Self {
        Self(format!("{}-{:08}", population_id, index))
    }
}

/// Configuration for population generation.
#[derive(Debug, Clone)]
pub struct PopulationConfig {
    /// Master seed for the entire population. Used to derive per-buyer seeds.
    pub population_seed: u64,
    /// Target total buyer count.
    pub target_count: usize,
    /// Whether to use correlated trait sampling (per segment).
    pub use_correlation: bool,
    /// If `use_correlation` is true, this maps segment id → correlation spec index.
    /// Default behavior builds a zero-correlation sampler when not provided.
    pub correlation_specs: BTreeMap<String, crate::correlated::CorrelationSpec>,
}

impl Default for PopulationConfig {
    fn default() -> Self {
        Self { population_seed: 42, target_count: 10_000, use_correlation: true, correlation_specs: BTreeMap::new() }
    }
}

/// The output of population generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntheticPopulationSnapshot {
    /// Total buyer count.
    pub buyer_count: usize,
    /// Population seed used (for reproducibility).
    pub population_seed: u64,
    /// Segment distribution summary.
    pub segment_distribution: BTreeMap<String, usize>,
    /// All buyers in order.
    pub buyers: Vec<Buyer>,
    /// Summary stats per segment.
    pub segment_summaries: BTreeMap<String, SegmentSummary>,
}

impl SyntheticPopulationSnapshot {
    /// Build a summary of key trait means for a segment.
    pub fn build_segment_summary(buyers: &[Buyer]) -> SegmentSummary {
        let n = buyers.len() as f64;
        let trait_sums: BTreeMap<String, f64> =
            buyers.iter().fold(BTreeMap::new(), |mut acc, b| {
                for (name, val) in &b.traits {
                    *acc.entry(name.clone()).or_insert(0.0) += val;
                }
                acc
            });
        let trait_means: BTreeMap<String, f64> =
            trait_sums.into_iter().map(|(k, v)| (k, v / n)).collect();
        SegmentSummary { buyer_count: buyers.len(), trait_means }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentSummary {
    pub buyer_count: usize,
    pub trait_means: BTreeMap<String, f64>,
}

/// Expand a set of blueprints into a `SyntheticPopulationSnapshot`.
pub struct PopulationGenerator {
    config: PopulationConfig,
}

impl PopulationGenerator {
    pub fn new(config: PopulationConfig) -> Self {
        Self { config }
    }

    /// Generate the full synthetic population.
    pub fn generate(&self, blueprints: &[SegmentBlueprint]) -> Result<SyntheticPopulationSnapshot, PopError> {
        if blueprints.is_empty() {
            return Err(PopError::NoBlueprints);
        }

        // First pass: count total target
        let total_target: usize = blueprints.iter().map(|bp| bp.target_count).sum();
        if total_target == 0 {
            return Err(PopError::NoBlueprints);
        }

        // Second pass: allocate buyers to blueprints proportionally
        let mut all_buyers: Vec<Buyer> = Vec::with_capacity(self.config.target_count);
        let mut segment_distribution: BTreeMap<String, usize> = BTreeMap::new();
        let mut global_idx = 0usize;

        // Master RNG seeded from population_seed
        let mut master_rng = ChaCha12Rng::seed_from_u64(self.config.population_seed);

        for blueprint in blueprints {
            let count = self.proportional_count(blueprint.target_count, total_target);
            let buyers = self.generate_for_blueprint(blueprint, count, &mut master_rng, global_idx);
            global_idx += buyers.len();
            segment_distribution.insert(blueprint.id.clone(), buyers.len());
            all_buyers.extend(buyers);
        }

        // Build segment summaries
        let mut segment_summaries: BTreeMap<String, SegmentSummary> = BTreeMap::new();
        for bp in blueprints {
            let bp_buyers: Vec<&Buyer> = all_buyers.iter().filter(|b| b.segment_id == bp.id).collect();
            if !bp_buyers.is_empty() {
                let summary = SyntheticPopulationSnapshot::build_segment_summary(
                    &bp_buyers.iter().map(|b| (*b).clone()).collect::<Vec<_>>()
                );
                segment_summaries.insert(bp.id.clone(), summary);
            }
        }

        Ok(SyntheticPopulationSnapshot {
            buyer_count: all_buyers.len(),
            population_seed: self.config.population_seed,
            segment_distribution,
            buyers: all_buyers,
            segment_summaries,
        })
    }

    fn proportional_count(&self, target: usize, total: usize) -> usize {
        let target_total = self.config.target_count;
        ((target as f64 / total as f64) * target_total as f64).round() as usize
    }

    fn generate_for_blueprint(
        &self,
        blueprint: &SegmentBlueprint,
        count: usize,
        master_rng: &mut ChaCha12Rng,
        start_idx: usize,
    ) -> Vec<Buyer> {
        let mut buyers = Vec::with_capacity(count);
        for i in 0..count {
            let buyer_seed = master_rng.next_u64();
            let mut buyer_rng = ChaCha12Rng::seed_from_u64(buyer_seed);
            let buyer_id = BuyerId::new(&blueprint.id, start_idx + i);

            // Sample traits
            let traits = self.sample_traits(blueprint, &mut buyer_rng);

            // Sample budget
            let budget = blueprint.budget.sample(&mut buyer_rng);

            // Sample channel
            let primary_channel = self.sample_primary_channel(blueprint, &mut buyer_rng);

            // Sample objections (activated probabilistically)
            let objections = blueprint
                .objections
                .iter()
                .filter(|obj| buyer_rng.gen::<f64>() < obj.activation_probability)
                .cloned()
                .collect();

            // Sample initial trust
            let initial_trust = blueprint.trust.initial_trust;

            buyers.push(Buyer {
                id: buyer_id,
                segment_id: blueprint.id.clone(),
                stage: blueprint.default_stage,
                traits,
                priors: blueprint.priors.clone(),
                budget,
                primary_channel,
                objections,
                trust: initial_trust,
                ..Default::default()
            });
        }
        buyers
    }

    fn sample_traits(&self, blueprint: &SegmentBlueprint, rng: &mut ChaCha12Rng) -> std::collections::BTreeMap<String, f64> {
        let builtins = [
            ("proof_hunger", &blueprint.proof_hunger),
            ("privacy_sensitivity", &blueprint.privacy_sensitivity),
            ("wearable_ownership", &blueprint.wearable_ownership),
            ("peptide_openness", &blueprint.peptide_openness),
            ("glp1_familiarity", &blueprint.glp1_familiarity),
            ("logging_tolerance", &blueprint.logging_tolerance),
            ("rigor_threshold", &blueprint.rigor_threshold),
        ];
        let mut traits: std::collections::BTreeMap<String, f64> = std::collections::BTreeMap::new();
        for (name, config) in builtins {
            let dist = match TraitDistribution::from_config(config) {
                Ok(d) => d,
                Err(_) => continue,
            };
            let val = dist.sample(rng).as_f64();
            traits.insert(name.to_string(), val);
        }
        for (name, config) in &blueprint.extra_traits {
            if let Ok(dist) = TraitDistribution::from_config(config) {
                traits.insert(name.clone(), dist.sample(rng).as_f64());
            }
        }
        traits
    }

    fn sample_primary_channel(&self, blueprint: &SegmentBlueprint, rng: &mut ChaCha12Rng) -> String {
        let weights = blueprint.normalized_channel_weights();
        if weights.is_empty() {
            return "organic_search".to_string();
        }
        let total: f64 = weights.values().sum();
        let r = rng.gen::<f64>() * total;
        let mut acc = 0.0;
        for (ch, w) in &weights {
            acc += w;
            if r <= acc {
                return ch.label().to_string();
            }
        }
        weights.keys().next().unwrap().label().to_string()
    }
}

#[derive(Debug, Clone, Error)]
pub enum PopError {
    #[error("no blueprints provided")]
    NoBlueprints,
    #[error("target count must be > 0")]
    ZeroTarget,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_blueprint(id: &str, target: usize) -> SegmentBlueprint {
        let mut bp = SegmentBlueprint::default();
        bp.id = id.to_string();
        bp.name = id.to_string();
        bp.target_count = target;
        bp
    }

    #[test]
    fn test_generate_proportional() {
        let bps = vec![make_blueprint("a", 1), make_blueprint("b", 1)];
        let gen = PopulationGenerator::new(PopulationConfig { population_seed: 0, target_count: 100, ..Default::default() });
        let snap = gen.generate(&bps).unwrap();
        // Both segments should get roughly equal buyers
        assert_eq!(snap.buyer_count, 100);
        let a = snap.segment_distribution.get("a").copied().unwrap_or(0);
        let b = snap.segment_distribution.get("b").copied().unwrap_or(0);
        assert!(a > 0 && b > 0);
    }

    #[test]
    fn test_reproducibility() {
        let bps = vec![make_blueprint("x", 50)];
        let gen1 = PopulationGenerator::new(PopulationConfig { population_seed: 7, target_count: 50, ..Default::default() });
        let snap1 = gen1.generate(&bps).unwrap();
        let gen2 = PopulationGenerator::new(PopulationConfig { population_seed: 7, target_count: 50, ..Default::default() });
        let snap2 = gen2.generate(&bps).unwrap();
        assert_eq!(snap1.buyers.len(), snap2.buyers.len());
        for (b1, b2) in snap1.buyers.iter().zip(snap2.buyers.iter()) {
            assert_eq!(b1.id.0, b2.id.0);
        }
    }

    #[test]
    fn test_unique_ids() {
        let bps = vec![make_blueprint("seg", 200)];
        let gen = PopulationGenerator::new(PopulationConfig { population_seed: 99, target_count: 200, ..Default::default() });
        let snap = gen.generate(&bps).unwrap();
        let ids: std::collections::HashSet<_> = snap.buyers.iter().map(|b| b.id.0.clone()).collect();
        assert_eq!(ids.len(), snap.buyers.len()); // all unique
    }

    #[test]
    fn test_no_blueprints_error() {
        let gen = PopulationGenerator::new(PopulationConfig::default());
        let result = gen.generate(&[]);
        assert!(result.is_err());
    }
}