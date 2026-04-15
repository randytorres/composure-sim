//! Correlated trait sampling via Cholesky decomposition — Task 13.
//!
//! Ensures sampled buyers feel realistic by enforcing covariance structure
//! between traits (e.g. high peptide_openness tends to co-occur with high
//! wearable_ownership).

use rand::Rng;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Correlation matrix: a symmetric positive-definite matrix of trait correlations.
/// Diagonal elements are always 1.0. Off-diagonal elements are in [-1, 1].
pub type CorrelationMatrix = Vec<Vec<f64>>;

/// Covariance matrix derived from correlation matrix and per-trait standard deviations.
pub type CovarianceMatrix = Vec<Vec<f64>>;

/// Specification of a correlation between two traits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraitCorrelation {
    pub trait_a: String,
    pub trait_b: String,
    /// Pearson correlation coefficient in [-1, 1].
    pub correlation: f64,
}

impl TraitCorrelation {
    pub fn new(trait_a: &str, trait_b: &str, correlation: f64) -> Self {
        Self { trait_a: trait_a.to_string(), trait_b: trait_b.to_string(), correlation }
    }
}

/// A set of trait correlations used to build a correlation matrix.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CorrelationSpec {
    /// List of trait names (in order). Their index in this list = row/col in the matrix.
    pub trait_names: Vec<String>,
    /// Correlation pairs. Unspecified off-diagonal entries default to 0.
    pub correlations: Vec<TraitCorrelation>,
}

impl CorrelationSpec {
    /// Build a `CorrelationMatrix` from this spec.
    pub fn build_correlation_matrix(&self) -> Result<CorrelationMatrix, CorrError> {
        let n = self.trait_names.len();
        let mut matrix: Vec<Vec<f64>> = vec![vec![0.0; n]; n];

        // Set diagonal to 1.0
        for i in 0..n {
            matrix[i][i] = 1.0;
        }

        // Build index map
        let index: std::collections::HashMap<&str, usize> =
            self.trait_names.iter().enumerate().map(|(i, t)| (t.as_str(), i)).collect();

        for corr in &self.correlations {
            let Some(&i) = index.get(corr.trait_a.as_str()) else {
                return Err(CorrError::UnknownTrait(corr.trait_a.clone()));
            };
            let Some(&j) = index.get(corr.trait_b.as_str()) else {
                return Err(CorrError::UnknownTrait(corr.trait_b.clone()));
            };
            if corr.correlation < -1.0 || corr.correlation > 1.0 {
                return Err(CorrError::OutOfRange(corr.correlation));
            }
            matrix[i][j] = corr.correlation;
            matrix[j][i] = corr.correlation;
        }

        Ok(matrix)
    }

    /// Build a `CovarianceMatrix` from this spec + standard deviations per trait.
    pub fn build_covariance_matrix(&self, stddevs: &[f64]) -> Result<CovarianceMatrix, CorrError> {
        if stddevs.len() != self.trait_names.len() {
            return Err(CorrError::LengthMismatch {
                expected: self.trait_names.len(),
                got: stddevs.len(),
            });
        }
        let corr = self.build_correlation_matrix()?;
        let n = self.trait_names.len();
        let mut cov = vec![vec![0.0; n]; n];
        for i in 0..n {
            for j in 0..n {
                cov[i][j] = corr[i][j] * stddevs[i] * stddevs[j];
            }
        }
        Ok(cov)
    }
}

/// Correlated trait sampler using Cholesky decomposition.
///
/// Usage:
/// 1. Create a `CorrelationSpec` with your trait names and correlations.
/// 2. Call `CorrelatedTraitSampler::new(&spec, &stddevs)`.
/// 3. Call `sample()` per buyer, passing a seeded RNG.
#[derive(Debug, Clone)]
pub struct CorrelatedTraitSampler {
    /// Cholesky lower-triangular factor L where C = L * L^T.
    /// n_traits × n_traits.
    cholesky: Vec<Vec<f64>>,
    trait_names: Vec<String>,
}

impl CorrelatedTraitSampler {
    /// Build a sampler from a `CorrelationSpec` and per-trait standard deviations.
    ///
    /// Internally builds the covariance matrix and computes its Cholesky decomposition.
    pub fn new(correlation_spec: &CorrelationSpec, stddevs: &[f64]) -> Result<Self, CorrError> {
        let cov = correlation_spec.build_covariance_matrix(stddevs)?;
        Self::from_covariance(&correlation_spec.trait_names, &cov)
    }

    /// Build directly from a covariance matrix.
    pub fn from_covariance(trait_names: &[String], cov: &[Vec<f64>]) -> Result<Self, CorrError> {
        let n = trait_names.len();
        if cov.len() != n || cov.iter().any(|row| row.len() != n) {
            return Err(CorrError::NotSquare { expected: n, got: cov.len() });
        }

        // Compute Cholesky decomposition L where C = L * L^T
        let mut l = vec![vec![0.0; n]; n];
        for i in 0..n {
            for j in 0..i {
                let mut sum = cov[i][j];
                for k in 0..j {
                    sum -= l[i][k] * l[j][k];
                }
                l[i][j] = if i == j {
                    let v = sum / l[j][j];
                    if v < 0.0 {
                        return Err(CorrError::NotPositiveDefinite);
                    }
                    v.sqrt()
                } else {
                    sum / l[j][j]
                };
            }
            // Diagonal
            let mut sum = cov[i][i];
            for k in 0..i {
                sum -= l[i][k] * l[i][k];
            }
            if sum <= 0.0 {
                return Err(CorrError::NotPositiveDefinite);
            }
            l[i][i] = sum.sqrt();
        }

        Ok(Self { cholesky: l, trait_names: trait_names.to_vec() })
    }

    /// Sample a vector of correlated standard-normal values, then transform them
    /// to have the target distribution via the provided mean/stddev per trait.
    ///
    /// Returns values in the same order as `trait_names`.
    pub fn sample<R: Rng>(&self, means: &[f64], rng: &mut R) -> Vec<f64> {
        let n = self.trait_names.len();
        // Sample n independent standard normals
        let z: Vec<f64> = (0..n).map(|_| {
            // Box-Muller
            let u1 = rng.gen::<f64>().max(1e-10);
            let u2 = rng.gen::<f64>();
            let mag = (-2.0 * u1.ln()).sqrt();
            mag * (2.0 * std::f64::consts::PI * u2).cos()
        }).collect();

        // Apply L * z to get correlated standard normals (stddevs baked into Cholesky)
        let correlated: Vec<f64> = (0..n).map(|i| {
            let mut sum = 0.0;
            for j in 0..=i {
                sum += self.cholesky[i][j] * z[j];
            }
            means[i] + sum
        }).collect();

        correlated
    }

    pub fn trait_count(&self) -> usize {
        self.trait_names.len()
    }
}

#[derive(Debug, Clone, Error)]
pub enum CorrError {
    #[error("unknown trait: `{0}`")]
    UnknownTrait(String),
    #[error("correlation {0} is out of range [-1, 1]")]
    OutOfRange(f64),
    #[error("length mismatch: expected {expected} stddevs, got {got}")]
    LengthMismatch { expected: usize, got: usize },
    #[error("covariance matrix is not square (expected {expected}×{expected}, got {got}×_)")]
    NotSquare { expected: usize, got: usize },
    #[error("matrix is not positive definite — check your correlation spec")]
    NotPositiveDefinite,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn test_identity_correlation() {
        // Zero off-diagonal = diagonal matrix => L = diag(stddev)
        let spec = CorrelationSpec {
            trait_names: vec!["a".to_string(), "b".to_string()],
            correlations: vec![],
        };
        let sampler = CorrelatedTraitSampler::new(&spec, &[0.1, 0.2]).unwrap();
        let mut rng = rand::rngs::StdRng::seed_from_u64(99);
        let samples = sampler.sample(&[0.5, 0.5], &mut rng);
        // With zero correlation, samples should be independent
        assert_eq!(samples.len(), 2);
    }

    #[test]
    fn test_positive_correlation() {
        let spec = CorrelationSpec {
            trait_names: vec!["openness".to_string(), "familiarity".to_string()],
            correlations: vec![TraitCorrelation::new("openness", "familiarity", 0.8)],
        };
        let sampler = CorrelatedTraitSampler::new(&spec, &[0.1, 0.1]).unwrap();
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        // Sample many and check correlation estimate
        let n = 10_000;
        let mut a_vals = Vec::with_capacity(n);
        let mut b_vals = Vec::with_capacity(n);
        for _ in 0..n {
            let sample = sampler.sample(&[0.5, 0.5], &mut rng);
            a_vals.push(sample[0]);
            b_vals.push(sample[1]);
        }
        let mean_a = a_vals.iter().sum::<f64>() / n as f64;
        let mean_b = b_vals.iter().sum::<f64>() / n as f64;
        let cov = a_vals.iter().zip(b_vals.iter()).map(|(a, b)| (a - mean_a) * (b - mean_b)).sum::<f64>() / n as f64;
        let var_a = a_vals.iter().map(|a| (a - mean_a).powi(2)).sum::<f64>() / n as f64;
        let var_b = b_vals.iter().map(|b| (b - mean_b).powi(2)).sum::<f64>() / n as f64;
        let est_corr = cov / (var_a.sqrt() * var_b.sqrt());
        assert!(est_corr > 0.6); // Should be close to 0.8
    }

    #[test]
    fn test_reproducibility() {
        let spec = CorrelationSpec {
            trait_names: vec!["a".to_string(), "b".to_string()],
            correlations: vec![TraitCorrelation::new("a", "b", 0.5)],
        };
        let sampler = CorrelatedTraitSampler::new(&spec, &[0.1, 0.1]).unwrap();
        let mut rng1 = rand::rngs::StdRng::seed_from_u64(123);
        let mut rng2 = rand::rngs::StdRng::seed_from_u64(123);
        let s1 = sampler.sample(&[0.5, 0.5], &mut rng1);
        let s2 = sampler.sample(&[0.5, 0.5], &mut rng2);
        assert_eq!(s1, s2);
    }

    #[test]
    fn test_not_positive_definite() {
        // Perfect anti-correlation of 1.0 on 2x2: [[1,-1],[-1,1]] is not positive definite
        let spec = CorrelationSpec {
            trait_names: vec!["a".to_string(), "b".to_string()],
            correlations: vec![TraitCorrelation::new("a", "b", 1.0)],
        };
        let result = CorrelatedTraitSampler::new(&spec, &[1.0, 1.0]);
        assert!(result.is_err());
    }
}