//! Trait distribution system — Task 12.
//!
//! Parametric trait distributions and a trait sampler that converts
//! `TraitDistributionConfig` (from `SegmentBlueprint`) into live values.

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TraitValue {
    Continuous(f64),
    Categorical(String),
}

impl TraitValue {
    pub fn as_f64(&self) -> f64 {
        match self {
            Self::Continuous(v) => *v,
            Self::Categorical(_) => 0.0, // caller maps categorical separately
        }
    }
}

/// A named trait distribution type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistributionType {
    Uniform,
    Normal,
    Beta,
    TruncatedNormal,
    Categorical,
}

impl DistributionType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "uniform" => Some(Self::Uniform),
            "normal" => Some(Self::Normal),
            "beta" => Some(Self::Beta),
            "truncated_normal" | "truncatednormal" => Some(Self::TruncatedNormal),
            "categorical" => Some(Self::Categorical),
            _ => None,
        }
    }
}

/// Trait distribution — resolved from `TraitDistributionConfig` at generation time.
#[derive(Debug, Clone)]
pub enum TraitDistribution {
    /// Uniform on [min, max].
    Uniform { min: f64, max: f64 },
    /// Normal with mean and stddev.
    Normal { mean: f64, stddev: f64 },
    /// Beta distribution with alpha and beta parameters.
    /// Range is [0, 1] internally; caller clamps.
    Beta { alpha: f64, beta: f64 },
    /// Truncated normal: samples from Normal(mean, stddev) then clamps to [lo, hi].
    TruncatedNormal {
        mean: f64,
        stddev: f64,
        lo: f64,
        hi: f64,
    },
    /// Categorical: pick from weighted options.
    Categorical { options: Vec<(String, f64)> },
}

impl TraitDistribution {
    /// Build from a `TraitDistributionConfig` (e.g. from a `SegmentBlueprint`).
    pub fn from_config(
        config: &crate::blueprint::TraitDistributionConfig,
    ) -> Result<Self, TraitError> {
        let dt = DistributionType::from_str(&config.distribution_type)
            .ok_or_else(|| TraitError::UnknownDistribution(config.distribution_type.clone()))?;

        match dt {
            DistributionType::Uniform => {
                let min = config.params.get("min").copied().unwrap_or(0.0);
                let max = config.params.get("max").copied().unwrap_or(1.0);
                if max < min {
                    return Err(TraitError::InvalidRange {
                        name: "Uniform".to_string(),
                        min,
                        max,
                    });
                }
                Ok(Self::Uniform { min, max })
            }
            DistributionType::Normal => {
                let mean = config.params.get("mean").copied().unwrap_or(0.5);
                let stddev = config.params.get("stddev").copied().unwrap_or(0.2);
                if stddev <= 0.0 {
                    return Err(TraitError::InvalidStddev {
                        name: "Normal".to_string(),
                        value: stddev,
                    });
                }
                Ok(Self::Normal { mean, stddev })
            }
            DistributionType::Beta => {
                let alpha = config.params.get("alpha").copied().unwrap_or(2.0);
                let beta = config.params.get("beta").copied().unwrap_or(2.0);
                if alpha <= 0.0 || beta <= 0.0 {
                    return Err(TraitError::InvalidShape {
                        name: "Beta".to_string(),
                        alpha,
                        beta,
                    });
                }
                Ok(Self::Beta { alpha, beta })
            }
            DistributionType::TruncatedNormal => {
                let mean = config.params.get("mean").copied().unwrap_or(0.5);
                let stddev = config.params.get("stddev").copied().unwrap_or(0.2);
                let lo = config.params.get("lo").copied().unwrap_or(0.0);
                let hi = config.params.get("hi").copied().unwrap_or(1.0);
                if stddev <= 0.0 {
                    return Err(TraitError::InvalidStddev {
                        name: "TruncatedNormal".to_string(),
                        value: stddev,
                    });
                }
                if hi < lo {
                    return Err(TraitError::InvalidRange {
                        name: "TruncatedNormal".to_string(),
                        min: lo,
                        max: hi,
                    });
                }
                Ok(Self::TruncatedNormal {
                    mean,
                    stddev,
                    lo,
                    hi,
                })
            }
            DistributionType::Categorical => {
                let options: Vec<(String, f64)> = config
                    .params
                    .iter()
                    .filter_map(|(k, v)| {
                        if k.starts_with("option_") {
                            Some((k.strip_prefix("option_").unwrap().to_string(), *v))
                        } else {
                            None
                        }
                    })
                    .collect();
                if options.is_empty() {
                    return Err(TraitError::CategoricalNeedsOptions);
                }
                if options.iter().any(|(_, weight)| *weight <= 0.0) {
                    return Err(TraitError::InvalidCategoricalWeight);
                }
                Ok(Self::Categorical { options })
            }
        }
    }

    /// Return (mean, stddev) estimate for each distribution type.
    /// Used by the population generator to seed the correlated sampler.
    pub fn mean_stddev(&self) -> (f64, f64) {
        match self {
            Self::Uniform { min, max } => {
                ((min + max) / 2.0, (max - min) / (2.0 * 1.732_f64.sqrt()))
            }
            Self::Normal { mean, stddev } => (*mean, *stddev),
            Self::Beta { alpha, beta } => {
                let denom = alpha + beta;
                (
                    alpha / denom,
                    (alpha * beta / (denom * denom * (denom + 1.0))).sqrt(),
                )
            }
            Self::TruncatedNormal { mean, stddev, .. } => (*mean, *stddev),
            // Categorical: use weighted average of option values as proxy mean
            Self::Categorical { options } => {
                if options.is_empty() {
                    return (0.5, 0.2);
                }
                let total: f64 = options.iter().map(|(_, w)| w).sum();
                let mean = options
                    .iter()
                    .map(|(name, w)| {
                        // Map categorical index to a continuous value (0-1 scale)
                        let idx = options.iter().position(|(n, _)| n == name).unwrap() as f64;
                        let normalized = idx / (options.len() - 1).max(1) as f64;
                        normalized * w / total
                    })
                    .sum::<f64>();
                (mean, 0.2)
            }
        }
    }

    /// Clamp a sampled continuous value into the distribution's supported range when known.
    pub fn clamp_continuous(&self, value: f64) -> f64 {
        match self {
            Self::Uniform { min, max } => value.clamp(*min, *max),
            Self::Beta { .. } => value.clamp(0.0, 1.0),
            Self::TruncatedNormal { lo, hi, .. } => value.clamp(*lo, *hi),
            Self::Normal { .. } => value,
            Self::Categorical { options } => {
                let max_index = options.len().saturating_sub(1) as f64;
                value.clamp(0.0, max_index.max(1.0))
            }
        }
    }

    /// Sample a single value from this distribution.
    pub fn sample<R: Rng>(&self, rng: &mut R) -> TraitValue {
        match self {
            Self::Uniform { min, max } => {
                TraitValue::Continuous(min + rng.gen::<f64>() * (max - min))
            }
            Self::Normal { mean, stddev } => {
                let n = rand_distr::Normal::new(*mean, *stddev)
                    .expect("Normal distribution created with validated params");
                TraitValue::Continuous(rng.sample(n))
            }
            Self::Beta { alpha, beta } => {
                let x = sample_beta_internal(rng, *alpha, *beta);
                TraitValue::Continuous(x)
            }
            Self::TruncatedNormal {
                mean,
                stddev,
                lo,
                hi,
            } => {
                let sample = rng.sample(rand_distr::Normal::new(*mean, *stddev).unwrap());
                TraitValue::Continuous(sample.clamp(*lo, *hi))
            }
            Self::Categorical { options } => {
                let total: f64 = options.iter().map(|(_, w)| w).sum();
                let r = rng.gen::<f64>() * total;
                let mut acc = 0.0;
                for (name, weight) in options {
                    acc += weight;
                    if r <= acc {
                        return TraitValue::Categorical(name.clone());
                    }
                }
                TraitValue::Categorical(options.last().unwrap().0.clone())
            }
        }
    }
}

/// Sample from Beta(alpha, beta) using the gamma method.
fn sample_beta_internal<R: Rng>(rng: &mut R, alpha: f64, beta: f64) -> f64 {
    let x = sample_gamma(rng, alpha);
    let y = sample_gamma(rng, beta);
    x / (x + y)
}

/// Sample from Gamma(shape, scale=1) using Marsaglia & Tsang's method.
fn sample_gamma<R: Rng>(rng: &mut R, shape: f64) -> f64 {
    if shape < 1.0 {
        // Use shape + 1 trick
        let g = sample_gamma(rng, shape + 1.0);
        let u = rng.gen::<f64>();
        return g * u.powf(1.0 / shape);
    }
    let d = shape - 1.0 / 3.0;
    let c = 1.0 / (9.0 * d).sqrt();
    loop {
        let mut x;
        let mut v;
        loop {
            x = rng.sample(rand_distr::Normal::new(0.0, 1.0).unwrap());
            v = 1.0 + c * x;
            if v > 0.0 {
                break;
            }
        }
        v = v.powi(3);
        let u = rng.gen::<f64>();
        if u < 1.0 - 0.0331 * (x * x).powi(2) {
            return d * v;
        }
        if u.ln() < 0.5 * x * x + d * (1.0 - v + v.ln()) {
            return d * v;
        }
    }
}

/// Shared trait sampler — converts a map of trait-name → `TraitDistributionConfig`
/// into sampled `TraitValue`s.
pub struct TraitSampler {
    distributions: BTreeMap<String, TraitDistribution>,
}

impl TraitSampler {
    /// Build a sampler from a map of trait name → `TraitDistributionConfig`.
    pub fn from_configs(
        configs: &BTreeMap<String, crate::blueprint::TraitDistributionConfig>,
    ) -> Result<Self, TraitError> {
        let distributions = configs
            .iter()
            .map(|(name, config)| {
                let dist = TraitDistribution::from_config(config)?;
                Ok((name.clone(), dist))
            })
            .collect::<Result<_, _>>()?;
        Ok(Self { distributions })
    }

    /// Sample all configured traits.
    pub fn sample_all<R: Rng>(&self, rng: &mut R) -> BTreeMap<String, TraitValue> {
        self.distributions
            .iter()
            .map(|(name, dist)| (name.clone(), dist.sample(rng)))
            .collect()
    }

    /// Sample a specific trait.
    pub fn sample<R: Rng>(&self, name: &str, rng: &mut R) -> Option<TraitValue> {
        self.distributions.get(name).map(|dist| dist.sample(rng))
    }
}

#[derive(Debug, Clone, Error)]
pub enum TraitError {
    #[error("unknown distribution type: `{0}`")]
    UnknownDistribution(String),
    #[error("categorical distribution requires at least one option")]
    CategoricalNeedsOptions,
    #[error("stddev must be positive, got {value} for trait `{name}`")]
    InvalidStddev { name: String, value: f64 },
    #[error("range is invalid for trait `{name}`: min={min}, max={max}")]
    InvalidRange { name: String, min: f64, max: f64 },
    #[error("shape parameters must be positive for trait `{name}`: alpha={alpha}, beta={beta}")]
    InvalidShape { name: String, alpha: f64, beta: f64 },
    #[error("categorical option weights must be positive")]
    InvalidCategoricalWeight,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn test_uniform_from_config() {
        let mut params = BTreeMap::new();
        params.insert("min".to_string(), 0.2);
        params.insert("max".to_string(), 0.8);
        let config = crate::blueprint::TraitDistributionConfig {
            distribution_type: "uniform".to_string(),
            params,
        };
        let dist = TraitDistribution::from_config(&config).unwrap();
        let mut rng = rand::thread_rng();
        let val = dist.sample(&mut rng);
        let f = val.as_f64();
        assert!(f >= 0.2 && f <= 0.8);
    }

    #[test]
    fn test_truncated_normal() {
        let mut params = BTreeMap::new();
        params.insert("mean".to_string(), 0.5);
        params.insert("stddev".to_string(), 0.1);
        params.insert("lo".to_string(), 0.0);
        params.insert("hi".to_string(), 1.0);
        let config = crate::blueprint::TraitDistributionConfig {
            distribution_type: "truncated_normal".to_string(),
            params,
        };
        let dist = TraitDistribution::from_config(&config).unwrap();
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        for _ in 0..100 {
            let val = dist.sample(&mut rng);
            let f = val.as_f64();
            assert!(f >= 0.0 && f <= 1.0);
        }
    }

    #[test]
    fn test_invalid_stddev_rejected() {
        let mut params = BTreeMap::new();
        params.insert("mean".to_string(), 0.5);
        params.insert("stddev".to_string(), 0.0);
        let config = crate::blueprint::TraitDistributionConfig {
            distribution_type: "normal".to_string(),
            params,
        };
        assert!(matches!(
            TraitDistribution::from_config(&config),
            Err(TraitError::InvalidStddev { .. })
        ));
    }

    #[test]
    fn test_trait_sampler() {
        let mut configs = BTreeMap::new();
        configs.insert(
            "proof_hunger".to_string(),
            crate::blueprint::TraitDistributionConfig {
                distribution_type: "uniform".to_string(),
                params: BTreeMap::new(),
            },
        );
        let sampler = TraitSampler::from_configs(&configs).unwrap();
        let values = sampler.sample_all(&mut rand::thread_rng());
        assert!(values.contains_key("proof_hunger"));
    }
}
