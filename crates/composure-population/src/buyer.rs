//! Buyer entity — Tasks 11–15.
//!
//! Individual buyer with sampled traits, memory state, and social identity.

use crate::blueprint::{Objection, Prior};
use serde::{Deserialize, Serialize};

/// Individual synthetic buyer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Buyer {
    /// Stable unique identifier.
    pub id: crate::population::BuyerId,
    /// Which segment this buyer was sampled from.
    pub segment_id: String,
    /// Current lifecycle stage.
    pub stage: crate::blueprint::SegmentStage,
    /// Sampled trait values (trait_name → value).
    pub traits: std::collections::BTreeMap<String, f64>,
    /// Prior beliefs about the product/category.
    pub priors: Vec<Prior>,
    /// Monthly budget in USD (sampled from segment budget distribution).
    pub budget: f64,
    /// Primary channel this buyer uses (sampled from channel preferences).
    pub primary_channel: String,

    // ─── Objections ────────────────────────────────────────────────────────
    /// Objections this buyer has activated (based on segment activation probability).
    pub objections: Vec<Objection>,

    // ─── Trust state ───────────────────────────────────────────────────────
    /// Current trust level [0, 1].
    pub trust: f64,

    // ─── Memory state ─────────────────────────────────────────────────────
    /// Prior exposures to marketing content (channel → count).
    pub exposure_history: std::collections::BTreeMap<String, usize>,
    /// Prior conversion attempts and outcomes.
    pub conversion_history: Vec<ConversionRecord>,
    /// Prior churn reasons.
    pub churn_history: Vec<ChurnRecord>,
    /// Trust changes over time (step → trust level).
    pub trust_trajectory: Vec<f64>,

    // ─── Social graph ─────────────────────────────────────────────────────
    /// Set of buyer IDs this buyer is connected to (serializable).
    pub connections: Vec<String>,
}

impl Default for Buyer {
    fn default() -> Self {
        Self {
            id: crate::population::BuyerId("default-00000000".to_string()),
            segment_id: "default".to_string(),
            stage: crate::blueprint::SegmentStage::CuriousObserver,
            traits: std::collections::BTreeMap::new(),
            priors: vec![],
            budget: 0.0,
            primary_channel: "organic_search".to_string(),
            objections: vec![],
            trust: 0.5,
            exposure_history: std::collections::BTreeMap::new(),
            conversion_history: vec![],
            churn_history: vec![],
            trust_trajectory: vec![0.5],
            connections: vec![],
        }
    }
}

impl Buyer {
    /// Record an exposure to a channel.
    pub fn record_exposure(&mut self, channel: &str) {
        *self.exposure_history.entry(channel.to_string()).or_insert(0) += 1;
    }

    /// Record a conversion attempt and its outcome.
    pub fn record_conversion(&mut self, outcome: ConversionOutcome) {
        self.conversion_history.push(ConversionRecord {
            step: self.trust_trajectory.len(),
            outcome,
        });
    }

    /// Record a churn event.
    pub fn record_churn(&mut self, reason: ChurnReason) {
        self.churn_history.push(ChurnRecord {
            step: self.trust_trajectory.len(),
            reason,
        });
    }

    /// Update trust and append to trajectory.
    pub fn update_trust(&mut self, delta: f64, trust_config: &crate::blueprint::TrustCalibration) {
        self.trust = (self.trust + delta).clamp(0.0, 1.0);
        if self.trust <= trust_config.trust_collapse_threshold {
            self.trust = 0.0; // trust collapsed
        }
        self.trust_trajectory.push(self.trust);
    }

    /// Total number of exposures across all channels.
    pub fn total_exposures(&self) -> usize {
        self.exposure_history.values().sum()
    }

    /// Check if any activated objection blocks conversion.
    pub fn has_blocking_objection(&self) -> bool {
        self.objections.iter().any(|o| o.severity > 0.7)
    }
}

/// Outcome of a conversion attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversionOutcome {
    Purchased,
    Abandonded,
    Deferred,
    Rejected,
}

/// Why a buyer churned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChurnReason {
    SideEffects,
    LackOfResults,
    Price,
    PrivacyConcern,
    Complexity,
    AlternativeProduct,
    MissingSupport,
}

/// A single conversion attempt record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionRecord {
    pub step: usize,
    pub outcome: ConversionOutcome,
}

/// A single churn event record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChurnRecord {
    pub step: usize,
    pub reason: ChurnReason,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_exposure() {
        let mut buyer = Buyer::default();
        buyer.record_exposure("tiktok");
        buyer.record_exposure("tiktok");
        assert_eq!(buyer.total_exposures(), 2);
    }

    #[test]
    fn test_trust_update() {
        let mut buyer = Buyer::default();
        let config = crate::blueprint::TrustCalibration::default();
        buyer.update_trust(0.1, &config);
        assert!((buyer.trust - 0.6).abs() < 1e-9);
        assert_eq!(buyer.trust_trajectory.len(), 2);
    }

    #[test]
    fn test_trust_collapse() {
        let mut buyer = Buyer::default();
        buyer.trust = 0.25;
        let config = crate::blueprint::TrustCalibration::default();
        buyer.update_trust(-0.5, &config);
        assert!(buyer.trust <= 0.0);
    }
}
