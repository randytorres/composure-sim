//! Segment blueprint schema — Task 11.
//!
//! Defines the reusable schema for a buyer segment with priors, traits,
//! channel preferences, objections, budget, trust, and product-friction tolerances.

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Stage a buyer is in within the segment lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentStage {
    /// Aware of the category but hasn't engaged deeply.
    CuriousObserver,
    /// Actively tracking product updates, reviews, comparisons.
    ActiveTracker,
    /// Has tried the product; evaluating outcomes.
    Evaluator,
    /// Ready to purchase or has purchased.
    Buyer,
    /// Satisfied and likely to repurchase or refer.
    Advocate,
    /// Dissatisfied; churn-risk or active churn.
    ChurnRiskSkeptic,
}

impl Default for SegmentStage {
    fn default() -> Self {
        Self::CuriousObserver
    }
}

/// Channel through which a buyer can be reached.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    Tiktok,
    Instagram,
    YouTube,
    Podcast,
    Reddit,
    Twitter,
    Newsletter,
    Email,
    Ppc,
    OrganicSearch,
    WordOfMouth,
    Influencer,
    CommunityForum,
    Custom(String),
}

impl Channel {
    /// Returns "other" for unknown variants.
    pub fn label(&self) -> &str {
        match self {
            Self::Custom(s) => s,
            _ => {
                let s = format!("{:?}", self).to_lowercase();
                &*Box::leak(s.into_boxed_str())
            }
        }
    }
}

/// Preference weight for a given channel (higher = more preferred).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelPreference {
    pub channel: Channel,
    /// Relative weight in [0.0, 1.0]. Unlisted channels default to 0.
    pub weight: f64,
    /// Expected reach per week (influencer reach estimate).
    pub reach_per_week: Option<f64>,
}

impl ChannelPreference {
    pub fn new(channel: Channel, weight: f64) -> Self {
        Self { channel, weight, reach_per_week: None }
    }
}

/// Type of objection a buyer might raise.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectionType {
    /// "Is it safe / backed by science?"
    Efficacy,
    /// "Is my data private?"
    Privacy,
    /// "I'll wait for peer reviews."
    Proof,
    /// "Too expensive / not worth it."
    Price,
    /// "Too complicated / too many steps."
    Complexity,
    /// "I tried something similar and it didn't work."
    PriorFailure,
    /// "My doctor / provider wouldn't approve."
    Authority,
    /// "Too many side effects reported."
    SideEffects,
    /// "My insurance won't cover it."
    Coverage,
    Custom(String),
}

/// A specific objection a segment harbors, with its activation threshold.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Objection {
    /// What kind of objection this is.
    pub objection_type: ObjectionType,
    /// Probability that a buyer in this segment raises this objection (0–1).
    pub activation_probability: f64,
    /// How strongly the objection blocks conversion when activated (0–1).
    pub severity: f64,
    /// Whether this objection can be resolved through proof/references.
    pub resolvable_via_social_proof: bool,
}

impl Objection {
    pub fn new(
        objection_type: ObjectionType,
        activation_probability: f64,
        severity: f64,
        resolvable_via_social_proof: bool,
    ) -> Self {
        Self { objection_type, activation_probability, severity, resolvable_via_social_proof }
    }
}

/// Product friction tolerances — how much friction a buyer tolerates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductFriction {
    /// How many onboarding steps are tolerable (1–10).
    pub onboarding_steps_tolerance: f64,
    /// How long setup can take in minutes.
    pub setup_time_tolerance_mins: f64,
    /// How many data points required before value is felt.
    pub logging_tolerance: f64,
    /// Minimum evidence quality (0–1) before trust forms.
    pub rigor_threshold: f64,
    /// Willingness to share biometric/health data (0 = none, 1 = full).
    pub data_sharing_tolerance: f64,
}

impl Default for ProductFriction {
    fn default() -> Self {
        Self {
            onboarding_steps_tolerance: 3.0,
            setup_time_tolerance_mins: 15.0,
            logging_tolerance: 5.0,
            rigor_threshold: 0.6,
            data_sharing_tolerance: 0.5,
        }
    }
}

/// A prior belief the buyer holds about the product or category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prior {
    /// Name of the belief (e.g., "GLP-1s cause muscle loss").
    pub belief: String,
    /// How strongly the buyer holds this belief [0,1].
    pub strength: f64,
    /// Whether the prior is positive (true) or negative (false).
    pub valence: bool,
    /// Source attribution ("influencer", "doctor", "reddit", etc.).
    pub source: Option<String>,
}

impl Prior {
    pub fn new(belief: &str, strength: f64, valence: bool) -> Self {
        Self { belief: belief.to_string(), strength, valence, source: None }
    }
}

/// Budget constraints for the buyer segment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    /// Monthly budget in USD.
    pub monthly_min: f64,
    pub monthly_max: f64,
    /// Whether price is a primary decision factor.
    pub price_sensitive: bool,
    /// Likely annual contract vs. month-to-month preference.
    pub prefers_annual: bool,
}

impl Budget {
    pub fn sample<R: Rng>(&self, rng: &mut R) -> f64 {
        let range = self.monthly_max - self.monthly_min;
        self.monthly_min + rng.gen::<f64>() * range
    }
}

/// Trust calibration for the segment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustCalibration {
    /// Base trust level at first exposure [0,1].
    pub initial_trust: f64,
    /// How quickly trust improves with positive interactions (slope).
    pub trust_recovery_rate: f64,
    /// How much a single bad experience penalizes trust.
    pub trust_penalty_per_incident: f64,
    /// Threshold below which trust is effectively broken.
    pub trust_collapse_threshold: f64,
}

impl Default for TrustCalibration {
    fn default() -> Self {
        Self {
            initial_trust: 0.5,
            trust_recovery_rate: 0.05,
            trust_penalty_per_incident: 0.1,
            trust_collapse_threshold: 0.2,
        }
    }
}

/// Distribution spec for a named trait.
///
/// Stored as a TOML/JSON-friendly config; resolved to a `TraitDistribution`
/// at population generation time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraitDistributionConfig {
    /// Distribution type: "uniform", "normal", "beta", "truncated_normal", "categorical".
    pub distribution_type: String,
    /// Parameters specific to the distribution type.
    pub params: BTreeMap<String, f64>,
}

impl Default for TraitDistributionConfig {
    fn default() -> Self {
        Self { distribution_type: "uniform".to_string(), params: BTreeMap::new() }
    }
}

/// The complete schema for a buyer segment.
///
/// `SegmentBlueprint` is the reusable input to the population generator.
/// It does not produce individual buyers — it defines the prior distribution
/// from which buyers are sampled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentBlueprint {
    /// Unique identifier for this segment.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Default stage for buyers entering this segment.
    pub default_stage: SegmentStage,

    // ─── Trait distributions ───────────────────────────────────────────────
    /// Distribution for proof_hunger: how much social proof is required before belief forms [0,1].
    pub proof_hunger: TraitDistributionConfig,
    /// Distribution for privacy_sensitivity: aversion to data sharing [0,1].
    pub privacy_sensitivity: TraitDistributionConfig,
    /// Distribution for wearable_ownership: already owns a wearable (0=no, 1=yes, continuous) [0,1].
    pub wearable_ownership: TraitDistributionConfig,
    /// Distribution for peptide_openness: openness to injectable peptides [0,1].
    pub peptide_openness: TraitDistributionConfig,
    /// Distribution for glp1_familiarity: prior knowledge of GLP-1 mechanisms [0,1].
    pub glp1_familiarity: TraitDistributionConfig,
    /// Distribution for logging_tolerance: how many data points before value is felt [0,10].
    pub logging_tolerance: TraitDistributionConfig,
    /// Distribution for rigor_threshold: minimum evidence quality before trust forms [0,1].
    pub rigor_threshold: TraitDistributionConfig,
    /// Additional domain-specific traits (arbitrary key-value).
    pub extra_traits: BTreeMap<String, TraitDistributionConfig>,

    // ─── Priors ───────────────────────────────────────────────────────────
    /// Prior beliefs the segment holds about the product/category.
    pub priors: Vec<Prior>,

    // ─── Channels ────────────────────────────────────────────────────────
    /// Ordered channel preferences (highest weight first).
    pub channel_preferences: Vec<ChannelPreference>,

    // ─── Objections ──────────────────────────────────────────────────────
    /// Objections this segment commonly raises.
    pub objections: Vec<Objection>,

    // ─── Budget & trust ──────────────────────────────────────────────────
    /// Budget range for this segment.
    pub budget: Budget,
    /// Trust calibration.
    pub trust: TrustCalibration,
    /// Product friction tolerances.
    pub friction: ProductFriction,

    // ─── Population sizing ────────────────────────────────────────────────
    /// Target buyer count for this blueprint.
    pub target_count: usize,
}

impl SegmentBlueprint {
    /// Returns a normalized map of channel → weight.
    pub fn normalized_channel_weights(&self) -> BTreeMap<Channel, f64> {
        let total: f64 = self.channel_preferences.iter().map(|cp| cp.weight).sum();
        if total == 0.0 {
            return BTreeMap::new();
        }
        self.channel_preferences.iter().map(|cp| (cp.channel.clone(), cp.weight / total)).collect()
    }

    /// Returns all trait names (built-in + extra) with their distribution configs.
    pub fn all_trait_names(&self) -> Vec<String> {
        let mut names = vec![
            "proof_hunger".to_string(),
            "privacy_sensitivity".to_string(),
            "wearable_ownership".to_string(),
            "peptide_openness".to_string(),
            "glp1_familiarity".to_string(),
            "logging_tolerance".to_string(),
            "rigor_threshold".to_string(),
        ];
        for key in self.extra_traits.keys() {
            names.push(key.clone());
        }
        names
    }

    /// Get the distribution config for a trait by name.
    pub fn trait_distribution(&self, name: &str) -> Option<&TraitDistributionConfig> {
        match name {
            "proof_hunger" => Some(&self.proof_hunger),
            "privacy_sensitivity" => Some(&self.privacy_sensitivity),
            "wearable_ownership" => Some(&self.wearable_ownership),
            "peptide_openness" => Some(&self.peptide_openness),
            "glp1_familiarity" => Some(&self.glp1_familiarity),
            "logging_tolerance" => Some(&self.logging_tolerance),
            "rigor_threshold" => Some(&self.rigor_threshold),
            _ => self.extra_traits.get(name),
        }
    }
}

impl Default for SegmentBlueprint {
    fn default() -> Self {
        Self {
            id: "default".to_string(),
            name: "Default Segment".to_string(),
            default_stage: SegmentStage::CuriousObserver,
            proof_hunger: TraitDistributionConfig::default(),
            privacy_sensitivity: TraitDistributionConfig::default(),
            wearable_ownership: TraitDistributionConfig::default(),
            peptide_openness: TraitDistributionConfig::default(),
            glp1_familiarity: TraitDistributionConfig::default(),
            logging_tolerance: TraitDistributionConfig::default(),
            rigor_threshold: TraitDistributionConfig::default(),
            extra_traits: BTreeMap::new(),
            priors: vec![],
            channel_preferences: vec![],
            objections: vec![],
            budget: Budget { monthly_min: 0.0, monthly_max: 1000.0, price_sensitive: true, prefers_annual: false },
            trust: TrustCalibration::default(),
            friction: ProductFriction::default(),
            target_count: 1000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_blueprint() {
        let bp = SegmentBlueprint::default();
        assert_eq!(bp.id, "default");
        assert_eq!(bp.default_stage, SegmentStage::CuriousObserver);
        assert_eq!(bp.target_count, 1000);
    }

    #[test]
    fn test_normalized_channel_weights() {
        let mut bp = SegmentBlueprint::default();
        bp.channel_preferences =
            vec![ChannelPreference::new(Channel::Tiktok, 0.6), ChannelPreference::new(Channel::Instagram, 0.4)];
        let weights = bp.normalized_channel_weights();
        assert!((weights.get(&Channel::Tiktok).unwrap() - 0.6).abs() < 1e-9);
        assert!((weights.get(&Channel::Instagram).unwrap() - 0.4).abs() < 1e-9);
    }

    #[test]
    fn test_all_trait_names() {
        let bp = SegmentBlueprint::default();
        let names = bp.all_trait_names();
        assert!(names.contains(&"proof_hunger".to_string()));
        assert!(names.contains(&"logging_tolerance".to_string()));
    }
}