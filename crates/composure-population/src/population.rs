//! Population generator — Task 14.
//!
//! Expands a set of `SegmentBlueprint`s into N buyers (10k–100k) with stable IDs
//! and seeded reproducibility.

use crate::correlated::CorrelatedTraitSampler;
use crate::traits::TraitDistribution;
use crate::{blueprint::SegmentBlueprint, buyer::Buyer};
use rand::{Rng, RngCore, SeedableRng};
use rand_chacha::ChaCha12Rng;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

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
    /// If `use_correlation` is true, maps segment id → correlation spec.
    /// When a segment has no spec here, falls back to independent sampling.
    pub correlation_specs: BTreeMap<String, crate::correlated::CorrelationSpec>,
}

impl Default for PopulationConfig {
    fn default() -> Self {
        Self {
            population_seed: 42,
            target_count: 10_000,
            use_correlation: true,
            correlation_specs: BTreeMap::new(),
        }
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
        SegmentSummary {
            buyer_count: buyers.len(),
            trait_means,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentSummary {
    pub buyer_count: usize,
    pub trait_means: BTreeMap<String, f64>,
}

/// Per-segment sampler state kept during generation.
struct SegmentSampler {
    /// Independent distributions for every trait defined on the blueprint.
    distributions: BTreeMap<String, TraitDistribution>,
    /// Correlated sampler (if use_correlation is true and a spec is available).
    correlated: Option<CorrelatedTraitSampler>,
    /// Trait names covered by the correlated sampler, in stable order.
    correlated_trait_names: Vec<String>,
    /// Per-trait means used by the correlated sampler.
    correlated_means: Vec<f64>,
}

impl SegmentSampler {
    /// Build a sampler for a blueprint. Returns an error if any trait config is
    /// invalid or the requested correlation spec is invalid.
    fn new(
        blueprint: &SegmentBlueprint,
        use_correlation: bool,
        spec: Option<&crate::correlated::CorrelationSpec>,
    ) -> Result<Self, PopError> {
        let mut distributions = BTreeMap::new();
        for trait_name in blueprint.all_trait_names() {
            let config = blueprint.trait_distribution(&trait_name).ok_or_else(|| {
                PopError::InvalidTraitConfig {
                    trait_name: trait_name.clone(),
                }
            })?;
            let dist = TraitDistribution::from_config(config).map_err(|_| {
                PopError::InvalidTraitConfig {
                    trait_name: trait_name.clone(),
                }
            })?;
            distributions.insert(trait_name, dist);
        }

        let correlated = if use_correlation {
            if let Some(spec) = spec {
                if spec.trait_names.is_empty() {
                    return Err(PopError::InvalidCorrelationSpec(
                        "correlation spec must include at least one trait".to_string(),
                    ));
                }
                let mut stddevs = Vec::with_capacity(spec.trait_names.len());
                let mut means = Vec::with_capacity(spec.trait_names.len());
                for trait_name in &spec.trait_names {
                    let dist = distributions.get(trait_name).ok_or_else(|| {
                        PopError::InvalidCorrelationSpec(format!(
                            "trait `{trait_name}` is not defined on blueprint `{}`",
                            blueprint.id
                        ))
                    })?;
                    let (mean, stddev) = dist.mean_stddev();
                    means.push(mean);
                    stddevs.push(stddev);
                }
                let sampler = CorrelatedTraitSampler::new(spec, &stddevs)
                    .map_err(|e| PopError::InvalidCorrelationSpec(e.to_string()))?;
                Some((sampler, spec.trait_names.clone(), means))
            } else {
                None
            }
        } else {
            None
        };

        let (correlated, correlated_trait_names, correlated_means) = correlated
            .map(|(sampler, trait_names, means)| (Some(sampler), trait_names, means))
            .unwrap_or_else(|| (None, Vec::new(), Vec::new()));

        Ok(Self {
            distributions,
            correlated,
            correlated_trait_names,
            correlated_means,
        })
    }

    /// Sample all traits for one buyer.
    fn sample(&self, rng: &mut ChaCha12Rng) -> BTreeMap<String, f64> {
        let mut traits = self
            .distributions
            .iter()
            .map(|(name, dist)| (name.clone(), dist.sample(rng).as_f64()))
            .collect::<BTreeMap<_, _>>();

        if let Some(ref sampler) = self.correlated {
            let correlated_values = sampler.sample(&self.correlated_means, rng);
            for (trait_name, value) in self
                .correlated_trait_names
                .iter()
                .zip(correlated_values.into_iter())
            {
                if let Some(dist) = self.distributions.get(trait_name) {
                    traits.insert(trait_name.clone(), dist.clamp_continuous(value));
                }
            }
        }

        traits
    }
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
    pub fn generate(
        &self,
        blueprints: &[SegmentBlueprint],
    ) -> Result<SyntheticPopulationSnapshot, PopError> {
        if blueprints.is_empty() {
            return Err(PopError::NoBlueprints);
        }

        let total_target: usize = blueprints.iter().map(|bp| bp.target_count).sum();
        if total_target == 0 {
            return Err(PopError::NoBlueprints);
        }

        // Pre-build per-segment samplers
        let segment_samplers: BTreeMap<String, SegmentSampler> = blueprints
            .iter()
            .map(|bp| {
                let spec = self.config.correlation_specs.get(&bp.id);
                let sampler = SegmentSampler::new(bp, self.config.use_correlation, spec)?;
                Ok((bp.id.clone(), sampler))
            })
            .collect::<Result<_, _>>()?;

        let mut all_buyers: Vec<Buyer> = Vec::with_capacity(self.config.target_count);
        let mut segment_distribution: BTreeMap<String, usize> = BTreeMap::new();
        let mut global_idx = 0usize;

        // Master RNG seeded from population_seed
        let mut master_rng = ChaCha12Rng::seed_from_u64(self.config.population_seed);

        for blueprint in blueprints {
            let count = self.proportional_count(blueprint.target_count, total_target);
            let sampler = segment_samplers
                .get(&blueprint.id)
                .expect("sampler built above");
            let buyers =
                self.generate_for_blueprint(blueprint, count, sampler, &mut master_rng, global_idx);
            global_idx += buyers.len();
            segment_distribution.insert(blueprint.id.clone(), buyers.len());
            all_buyers.extend(buyers);
        }

        // Build segment summaries
        let mut segment_summaries: BTreeMap<String, SegmentSummary> = BTreeMap::new();
        for bp in blueprints {
            let bp_buyers: Vec<&Buyer> = all_buyers
                .iter()
                .filter(|b| b.segment_id == bp.id)
                .collect();
            if !bp_buyers.is_empty() {
                let summary = SyntheticPopulationSnapshot::build_segment_summary(
                    &bp_buyers.iter().map(|b| (*b).clone()).collect::<Vec<_>>(),
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
        sampler: &SegmentSampler,
        master_rng: &mut ChaCha12Rng,
        start_idx: usize,
    ) -> Vec<Buyer> {
        let mut buyers = Vec::with_capacity(count);
        for i in 0..count {
            let buyer_seed = master_rng.next_u64();
            let mut buyer_rng = ChaCha12Rng::seed_from_u64(buyer_seed);
            let buyer_id = BuyerId::new(&blueprint.id, start_idx + i);

            // Sample traits (correlated or independent based on config)
            let traits = sampler.sample(&mut buyer_rng);

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

    fn sample_primary_channel(
        &self,
        blueprint: &SegmentBlueprint,
        rng: &mut ChaCha12Rng,
    ) -> String {
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
    #[error("invalid trait config for `{trait_name}`")]
    InvalidTraitConfig { trait_name: String },
    #[error("invalid correlation spec: {0}")]
    InvalidCorrelationSpec(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::correlated::{CorrelationSpec, TraitCorrelation};

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
        let gen = PopulationGenerator::new(PopulationConfig {
            population_seed: 0,
            target_count: 100,
            ..Default::default()
        });
        let snap = gen.generate(&bps).unwrap();
        assert_eq!(snap.buyer_count, 100);
        let a = snap.segment_distribution.get("a").copied().unwrap_or(0);
        let b = snap.segment_distribution.get("b").copied().unwrap_or(0);
        assert!(a > 0 && b > 0);
    }

    #[test]
    fn test_reproducibility() {
        let bps = vec![make_blueprint("x", 50)];
        let gen1 = PopulationGenerator::new(PopulationConfig {
            population_seed: 7,
            target_count: 50,
            ..Default::default()
        });
        let snap1 = gen1.generate(&bps).unwrap();
        let gen2 = PopulationGenerator::new(PopulationConfig {
            population_seed: 7,
            target_count: 50,
            ..Default::default()
        });
        let snap2 = gen2.generate(&bps).unwrap();
        assert_eq!(snap1.buyers.len(), snap2.buyers.len());
        for (b1, b2) in snap1.buyers.iter().zip(snap2.buyers.iter()) {
            assert_eq!(b1.id.0, b2.id.0);
        }
    }

    #[test]
    fn test_unique_ids() {
        let bps = vec![make_blueprint("seg", 200)];
        let gen = PopulationGenerator::new(PopulationConfig {
            population_seed: 99,
            target_count: 200,
            ..Default::default()
        });
        let snap = gen.generate(&bps).unwrap();
        let ids: std::collections::HashSet<_> =
            snap.buyers.iter().map(|b| b.id.0.clone()).collect();
        assert_eq!(ids.len(), snap.buyers.len());
    }

    #[test]
    fn test_no_blueprints_error() {
        let gen = PopulationGenerator::new(PopulationConfig::default());
        let result = gen.generate(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_correlated_sampling_is_used() {
        let bp = make_blueprint("corr", 4_000);
        let spec = CorrelationSpec {
            trait_names: vec!["proof_hunger".to_string(), "glp1_familiarity".to_string()],
            correlations: vec![TraitCorrelation::new(
                "proof_hunger",
                "glp1_familiarity",
                0.8,
            )],
        };
        let mut cfg = PopulationConfig::default();
        cfg.use_correlation = true;
        cfg.target_count = 4_000;
        cfg.correlation_specs.insert("corr".to_string(), spec);

        let gen = PopulationGenerator::new(cfg);
        let snap = gen.generate(&[bp]).unwrap();

        let hunger: Vec<_> = snap
            .buyers
            .iter()
            .map(|b| b.traits.get("proof_hunger").copied().unwrap_or(0.0))
            .collect();
        let fam: Vec<_> = snap
            .buyers
            .iter()
            .map(|b| b.traits.get("glp1_familiarity").copied().unwrap_or(0.0))
            .collect();

        let n = hunger.len() as f64;
        let h_mean = hunger.iter().sum::<f64>() / n;
        let f_mean = fam.iter().sum::<f64>() / n;
        let cov: f64 = hunger
            .iter()
            .zip(fam.iter())
            .map(|(h, f)| (h - h_mean) * (f - f_mean))
            .sum::<f64>()
            / n;
        assert!(
            cov > 0.01,
            "correlated traits should have positive covariance"
        );
    }

    #[test]
    fn test_invalid_extra_trait_config_returns_error() {
        let mut bp = make_blueprint("invalid", 10);
        bp.extra_traits.insert(
            "bad_trait".to_string(),
            crate::blueprint::TraitDistributionConfig {
                distribution_type: "normal".to_string(),
                params: BTreeMap::from([("mean".to_string(), 0.5), ("stddev".to_string(), 0.0)]),
            },
        );
        let gen = PopulationGenerator::new(PopulationConfig::default());
        assert!(matches!(
            gen.generate(&[bp]),
            Err(PopError::InvalidTraitConfig { trait_name }) if trait_name == "bad_trait"
        ));
    }
}
