//! Market simulation schemas: buyer archetypes, population config, campaign variants,
//! events, outcomes, and validation.

use rand::distributions::WeightedIndex;
use rand::Rng;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Buyer archetypes
// ---------------------------------------------------------------------------

/// Buyer archetype — behavioral pattern that drives transition probabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuyerArchetype {
    /// High purchase intent, short consideration window, high LTV
    HighIntent,
    /// Browsers with long consideration, low urgency, moderate conversion
    Browsers,
    /// Price-sensitive, respond to deals/discounts, lower LTV
    DealSeekers,
    /// Loyal repeat buyers, high retention, viral
    Loyalists,
    /// Dormant / re-engagement target, low initial propensity
    Dormant,
}

impl BuyerArchetype {
    /// Base click probability multiplier for this archetype.
    pub fn base_ctr(&self) -> f64 {
        match self {
            BuyerArchetype::HighIntent => 0.12,
            BuyerArchetype::Browsers => 0.05,
            BuyerArchetype::DealSeekers => 0.08,
            BuyerArchetype::Loyalists => 0.10,
            BuyerArchetype::Dormant => 0.02,
        }
    }

    /// Base signup probability once aware.
    pub fn base_signup_rate(&self) -> f64 {
        match self {
            BuyerArchetype::HighIntent => 0.40,
            BuyerArchetype::Browsers => 0.15,
            BuyerArchetype::DealSeekers => 0.25,
            BuyerArchetype::Loyalists => 0.50,
            BuyerArchetype::Dormant => 0.05,
        }
    }

    /// Base activation rate given signup.
    pub fn base_activation_rate(&self) -> f64 {
        match self {
            BuyerArchetype::HighIntent => 0.70,
            BuyerArchetype::Browsers => 0.30,
            BuyerArchetype::DealSeekers => 0.45,
            BuyerArchetype::Loyalists => 0.80,
            BuyerArchetype::Dormant => 0.10,
        }
    }

    /// Weekly retention probability given activation.
    pub fn base_weekly_retention(&self) -> f64 {
        match self {
            BuyerArchetype::HighIntent => 0.85,
            BuyerArchetype::Browsers => 0.60,
            BuyerArchetype::DealSeekers => 0.55,
            BuyerArchetype::Loyalists => 0.92,
            BuyerArchetype::Dormant => 0.40,
        }
    }

    /// Weekly churn probability given retention.
    pub fn base_weekly_churn(&self) -> f64 {
        1.0 - self.base_weekly_retention()
    }

    /// Base referral/share probability given activation.
    pub fn base_share_rate(&self) -> f64 {
        match self {
            BuyerArchetype::HighIntent => 0.05,
            BuyerArchetype::Browsers => 0.03,
            BuyerArchetype::DealSeekers => 0.02,
            BuyerArchetype::Loyalists => 0.15,
            BuyerArchetype::Dormant => 0.01,
        }
    }

    /// Average LTV in cents for a converted buyer of this archetype.
    pub fn avg_ltv_cents(&self) -> f64 {
        match self {
            BuyerArchetype::HighIntent => 8_000.0,
            BuyerArchetype::Browsers => 3_500.0,
            BuyerArchetype::DealSeekers => 2_000.0,
            BuyerArchetype::Loyalists => 15_000.0,
            BuyerArchetype::Dormant => 1_000.0,
        }
    }
}

/// All archetype variants in display order.
pub const ARCHETYPE_VARIANTS: [BuyerArchetype; 5] = [
    BuyerArchetype::HighIntent,
    BuyerArchetype::Browsers,
    BuyerArchetype::DealSeekers,
    BuyerArchetype::Loyalists,
    BuyerArchetype::Dormant,
];

// ---------------------------------------------------------------------------
// Population config
// ---------------------------------------------------------------------------

/// Probability weights used to sample archetype during population generation.
/// Must sum to 1.0 (validated at load time).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchetypeWeights {
    pub high_intent: f64,
    pub browsers: f64,
    pub deal_seekers: f64,
    pub loyalists: f64,
    pub dormant: f64,
}

impl Default for ArchetypeWeights {
    fn default() -> Self {
        Self {
            high_intent: 0.10,
            browsers: 0.25,
            deal_seekers: 0.20,
            loyalists: 0.15,
            dormant: 0.30,
        }
    }
}

impl ArchetypeWeights {
    /// Convert to flat slice for WeightedAliasIndex.
    fn to_slice(&self) -> Vec<f64> {
        vec![
            self.high_intent,
            self.browsers,
            self.deal_seekers,
            self.loyalists,
            self.dormant,
        ]
    }

    /// Build a WeightedIndex sampler from these weights.
    pub fn sampler<R: Rng>(&self, _rng: &mut R) -> WeightedIndex<f64> {
        WeightedIndex::new(self.to_slice()).expect("archetype weights must sum to 1.0")
    }

    /// Normalize weights so they sum to 1.0.
    pub fn normalized(&self) -> Self {
        let sum = self.high_intent + self.browsers + self.deal_seekers + self.loyalists + self.dormant;
        if sum == 0.0 {
            return Self::default();
        }
        Self {
            high_intent: self.high_intent / sum,
            browsers: self.browsers / sum,
            deal_seekers: self.deal_seekers / sum,
            loyalists: self.loyalists / sum,
            dormant: self.dormant / sum,
        }
    }
}

/// Configuration for the synthetic buyer population.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntheticPopulationConfig {
    /// Number of buyers to simulate. Range: 1..=1_000_000.
    pub population_size: usize,
    /// Number of time steps (e.g. days) to simulate. Range: 1..=3650.
    pub time_steps: usize,
    /// Random seed for deterministic reproducibility.
    /// A value of 0 is replaced with a fixed fallback seed.
    pub seed: u64,
    /// Archetype probability weights for population sampling.
    #[serde(default)]
    pub archetype_weights: ArchetypeWeights,
    /// Fraction of buyer records to include in output (0.0..=1.0).
    /// Default 0.01 (1%). Set to 1.0 to include all.
    #[serde(default = "default_sample_rate")]
    pub sample_rate: f64,
}

fn default_sample_rate() -> f64 {
    0.01
}

impl Default for SyntheticPopulationConfig {
    fn default() -> Self {
        Self {
            population_size: 1_000,
            time_steps: 30,
            seed: 42,
            archetype_weights: ArchetypeWeights::default(),
            sample_rate: 0.01,
        }
    }
}

impl SyntheticPopulationConfig {
    /// Return the effective seed (0 is replaced with 0xdeadbeef).
    pub fn effective_seed(&self) -> u64 {
        if self.seed == 0 {
            0xdeadbeef
        } else {
            self.seed
        }
    }
}

// ---------------------------------------------------------------------------
// Campaign variant
// ---------------------------------------------------------------------------

/// Channel attribution weights (must sum to 1.0).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelWeights {
    pub organic: f64,
    pub paid_search: f64,
    pub paid_social: f64,
    pub email: f64,
    pub referral: f64,
}

impl Default for ChannelWeights {
    fn default() -> Self {
        Self {
            organic: 0.20,
            paid_search: 0.30,
            paid_social: 0.25,
            email: 0.15,
            referral: 0.10,
        }
    }
}

impl ChannelWeights {
    pub fn to_slice(&self) -> Vec<f64> {
        vec![
            self.organic,
            self.paid_search,
            self.paid_social,
            self.email,
            self.referral,
        ]
    }

    pub fn sampler<R: Rng>(&self, _rng: &mut R) -> WeightedIndex<f64> {
        WeightedIndex::new(self.to_slice()).expect("channel weights must sum to 1.0")
    }
}

/// Creative multiplier by archetype for a given variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreativeMultipliers {
    pub high_intent: f64,
    pub browsers: f64,
    pub deal_seekers: f64,
    pub loyalists: f64,
    pub dormant: f64,
}

impl Default for CreativeMultipliers {
    fn default() -> Self {
        Self {
            high_intent: 1.0,
            browsers: 1.0,
            deal_seekers: 1.0,
            loyalists: 1.0,
            dormant: 1.0,
        }
    }
}

impl CreativeMultipliers {
    /// Get the creative multiplier for a given archetype.
    pub fn get(&self, archetype: BuyerArchetype) -> f64 {
        match archetype {
            BuyerArchetype::HighIntent => self.high_intent,
            BuyerArchetype::Browsers => self.browsers,
            BuyerArchetype::DealSeekers => self.deal_seekers,
            BuyerArchetype::Loyalists => self.loyalists,
            BuyerArchetype::Dormant => self.dormant,
        }
    }
}

/// A single campaign variant (e.g. Control vs Treatment).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CampaignVariant {
    /// Unique identifier for this variant.
    pub variant_id: String,
    /// Total spend budget for this variant across all time steps.
    pub spend_budget: f64,
    /// Channel mix weights.
    #[serde(default)]
    pub channel_weights: ChannelWeights,
    /// Creative/positioning multiplier per archetype.
    #[serde(default)]
    pub creative_multipliers: CreativeMultipliers,
    /// Base awareness generation rate per time step per buyer.
    #[serde(default = "default_awareness_rate")]
    pub awareness_rate: f64,
    /// Average impressions per buyer per time step when exposed.
    #[serde(default = "default_impressions")]
    pub impressions_per_exposure: f64,
}

fn default_awareness_rate() -> f64 {
    0.05
}

fn default_impressions() -> f64 {
    2.0
}

/// Market-level simulation configuration: population + one or more variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketSimulationConfig {
    /// Synthetic population parameters.
    #[serde(flatten)]
    pub population: SyntheticPopulationConfig,
    /// Campaign variants to simulate.
    pub variants: Vec<CampaignVariant>,
}

impl Default for MarketSimulationConfig {
    fn default() -> Self {
        Self {
            population: SyntheticPopulationConfig::default(),
            variants: vec![
                CampaignVariant {
                    variant_id: "control".into(),
                    spend_budget: 10_000.0,
                    ..Default::default()
                },
                CampaignVariant {
                    variant_id: "treatment".into(),
                    spend_budget: 15_000.0,
                    creative_multipliers: CreativeMultipliers {
                        high_intent: 1.2,
                        browsers: 1.1,
                        deal_seekers: 1.3,
                        loyalists: 1.1,
                        dormant: 1.0,
                    },
                    ..Default::default()
                },
            ],
        }
    }
}

// ---------------------------------------------------------------------------
// Buyer state
// ---------------------------------------------------------------------------

/// Channel through which a buyer was exposed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    Organic,
    PaidSearch,
    PaidSocial,
    Email,
    Referral,
}

/// A record of exposure for a specific buyer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposureRecord {
    pub timestep: usize,
    pub channel: Channel,
    pub impressions: f64,
    pub spend_at_t: f64,
}

/// The mutable state of a single buyer across the simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuyerState {
    pub buyer_id: usize,
    pub archetype: BuyerArchetype,
    /// Whether the buyer has been made aware of the product.
    pub aware: bool,
    /// Whether the buyer is actively considering (has been aware for some time).
    pub considering: bool,
    /// Time step when the buyer signed up (-1 if never).
    pub signup_t: i32,
    /// Time step when the buyer activated (-1 if never).
    pub activated_t: i32,
    /// Time step when the buyer churned (-1 if still active).
    pub churned_t: i32,
    /// Number of successful referrals made.
    pub referral_count: usize,
    /// Exposure history for attribution.
    pub exposures: Vec<ExposureRecord>,
}

impl BuyerState {
    pub fn new(buyer_id: usize, archetype: BuyerArchetype) -> Self {
        Self {
            buyer_id,
            archetype,
            aware: false,
            considering: false,
            signup_t: -1,
            activated_t: -1,
            churned_t: -1,
            referral_count: 0,
            exposures: Vec::new(),
        }
    }

    pub fn is_active(&self) -> bool {
        self.churned_t < 0
    }

    pub fn is_retained(&self) -> bool {
        self.activated_t >= 0 && self.churned_t < 0
    }
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// A conversion event fired during simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversionEventType {
    Signup,
    Activation,
    Purchase,
    Referral,
}

/// A conversion event fired during simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionEvent {
    pub buyer_id: usize,
    pub timestep: usize,
    pub event_type: ConversionEventType,
    /// Revenue generated in cents (0 for Signup/Referral).
    pub revenue_cents: i64,
}

// ---------------------------------------------------------------------------
// Scoring
// ---------------------------------------------------------------------------

/// Per-buyer transition probabilities scored at a given time step.
///
/// Used for analytics and reporting (separate from actual transition
/// decisions which share the same seeded RNG).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuyerScores {
    /// Probability of awareness/conversion event (click).
    pub click_probability: f64,
    /// Probability of signup given considering state.
    pub signup_probability: f64,
    /// Probability of activation given signup.
    pub activation_probability: f64,
    /// Probability of remaining active given activation.
    pub retention_probability: f64,
    /// Probability of churn given activation.
    pub churn_probability: f64,
    /// Probability of making a referral given activation.
    pub share_probability: f64,
}

// ---------------------------------------------------------------------------
// Outcomes
// ---------------------------------------------------------------------------

/// Outcome for a single buyer at simulation end.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuyerOutcome {
    pub buyer_id: usize,
    pub archetype: BuyerArchetype,
    pub reached_signup: bool,
    pub reached_activation: bool,
    pub churned: bool,
    pub referral_count: usize,
    /// Cumulative lifetime value in cents.
    pub lifetime_value_cents: f64,
    /// Timestep of signup (-1 if none).
    pub signup_t: i32,
    /// Timestep of activation (-1 if none).
    pub activated_t: i32,
}

impl BuyerOutcome {
    pub fn from_state(state: &BuyerState, revenue_per_referral_cents: f64) -> Self {
        let ltv = if state.activated_t >= 0 {
            state.archetype.avg_ltv_cents()
                + (state.referral_count as f64) * revenue_per_referral_cents
        } else {
            0.0
        };
        Self {
            buyer_id: state.buyer_id,
            archetype: state.archetype,
            reached_signup: state.signup_t >= 0,
            reached_activation: state.activated_t >= 0,
            churned: state.churned_t >= 0,
            referral_count: state.referral_count,
            lifetime_value_cents: ltv,
            signup_t: state.signup_t,
            activated_t: state.activated_t,
        }
    }
}

/// A cohort group defined by archetype + signup timing bucket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohortOutcome {
    /// Human-readable cohort key.
    pub segment_key: String,
    /// Archetype of this cohort.
    pub archetype: BuyerArchetype,
    /// Number of buyers in this cohort.
    pub buyer_count: usize,
    /// Fraction of cohort that signed up.
    pub signup_rate: f64,
    /// Fraction of cohort that activated.
    pub activation_rate: f64,
    /// Fraction of cohort that churned (of those activated).
    pub churn_rate: f64,
    /// Average LTV across cohort in cents.
    pub avg_ltv_cents: f64,
    /// Total revenue from this cohort in cents.
    pub total_revenue_cents: f64,
}

/// Aggregate market-level totals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketTotals {
    pub total_buyers: usize,
    pub total_signups: usize,
    pub total_activations: usize,
    pub total_churns: usize,
    pub total_referrals: usize,
    pub total_revenue_cents: f64,
    pub market_ctr: f64,
    pub market_cvr: f64,
    pub market_ltv: f64,
}

/// Complete output of a market simulation run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketSimulationResult {
    /// Sampled buyer outcomes (controlled by sample_rate).
    pub buyers: Vec<BuyerOutcome>,
    /// Cohort-level aggregates.
    pub cohorts: Vec<CohortOutcome>,
    /// Market-wide totals.
    pub market_totals: MarketTotals,
    /// SHA-256 digest of the canonical JSON config for reproducibility verification.
    pub config_digest: String,
    /// Number of variants simulated.
    pub variant_count: usize,
    /// Time steps simulated.
    pub time_steps: usize,
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// A single validation error with field path and message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

/// Validatable trait for config structs.
pub trait Validate {
    fn validate(&self) -> Vec<ValidationError>;
}

fn invalid_field(field: &str, message: &str) -> ValidationError {
    ValidationError {
        field: field.to_string(),
        message: message.to_string(),
    }
}

impl Validate for SyntheticPopulationConfig {
    fn validate(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        if self.population_size == 0 {
            errors.push(invalid_field("population_size", "must be >= 1"));
        }
        if self.population_size > 1_000_000 {
            errors.push(invalid_field(
                "population_size",
                "must be <= 1_000_000",
            ));
        }

        if self.time_steps == 0 {
            errors.push(invalid_field("time_steps", "must be >= 1"));
        }
        if self.time_steps > 3650 {
            errors.push(invalid_field("time_steps", "must be <= 3650"));
        }

        let w = &self.archetype_weights;
        let sum = w.high_intent + w.browsers + w.deal_seekers + w.loyalists + w.dormant;
        if (sum - 1.0).abs() > 0.001 {
            errors.push(invalid_field(
                "archetype_weights",
                &format!("weights must sum to 1.0, got {sum:.6}"),
            ));
        }

        if self.sample_rate < 0.0 || self.sample_rate > 1.0 {
            errors.push(invalid_field(
                "sample_rate",
                "must be between 0.0 and 1.0",
            ));
        }

        errors
    }
}

impl Validate for CampaignVariant {
    fn validate(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        if self.variant_id.trim().is_empty() {
            errors.push(invalid_field("variant_id", "must be non-empty"));
        }

        if self.spend_budget <= 0.0 {
            errors.push(invalid_field("spend_budget", "must be > 0"));
        }

        let cw = &self.channel_weights;
        let sum = cw.organic + cw.paid_search + cw.paid_social + cw.email + cw.referral;
        if (sum - 1.0).abs() > 0.001 {
            errors.push(invalid_field(
                "channel_weights",
                &format!("weights must sum to 1.0, got {sum:.6}"),
            ));
        }

        if self.awareness_rate < 0.0 || self.awareness_rate > 1.0 {
            errors.push(invalid_field(
                "awareness_rate",
                "must be between 0.0 and 1.0",
            ));
        }

        errors
    }
}

impl Validate for MarketSimulationConfig {
    fn validate(&self) -> Vec<ValidationError> {
        let mut errors = self.population.validate();
        for (i, variant) in self.variants.iter().enumerate() {
            for mut err in variant.validate() {
                err.field = format!("variants[{i}].{}", err.field);
                errors.push(err);
            }
        }
        if self.variants.is_empty() {
            errors.push(invalid_field("variants", "must have at least one variant"));
        }
        errors
    }
}

// ---------------------------------------------------------------------------
// Config digest (reproducibility hash)
// ---------------------------------------------------------------------------

/// Compute a short SHA-256 hex digest of a config for reproducibility tracking.
pub fn config_digest(config: &MarketSimulationConfig) -> String {
    use sha2::Digest;
    let json = serde_json::to_string(config).expect("config must be serializable");
    let mut hasher = sha2::Sha256::new();
    hasher.update(json.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)[..16].to_string()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archetype_weights_default_sums_to_one() {
        let w = ArchetypeWeights::default();
        let sum = w.high_intent + w.browsers + w.deal_seekers + w.loyalists + w.dormant;
        assert!((sum - 1.0).abs() < 1e-9);
    }

    #[test]
    fn archetype_weights_normalized() {
        let w = ArchetypeWeights {
            high_intent: 10.0,
            browsers: 20.0,
            deal_seekers: 0.0,
            loyalists: 0.0,
            dormant: 0.0,
        };
        let n = w.normalized();
        let sum = n.high_intent + n.browsers + n.deal_seekers + n.loyalists + n.dormant;
        assert!((sum - 1.0).abs() < 1e-9);
        assert!((n.high_intent - 0.333_333).abs() < 1e-6);
        assert!((n.browsers - 0.666_667).abs() < 1e-6);
    }

    #[test]
    fn channel_weights_default_sums_to_one() {
        let cw = ChannelWeights::default();
        let sum = cw.organic + cw.paid_search + cw.paid_social + cw.email + cw.referral;
        assert!((sum - 1.0).abs() < 1e-9);
    }

    #[test]
    fn population_config_validate_rejects_zero_size() {
        let config = SyntheticPopulationConfig {
            population_size: 0,
            ..Default::default()
        };
        let errors = config.validate();
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.field == "population_size"));
    }

    #[test]
    fn population_config_validate_rejects_unnormalized_weights() {
        let config = SyntheticPopulationConfig {
            archetype_weights: ArchetypeWeights {
                high_intent: 0.6,
                browsers: 0.6,
                deal_seekers: 0.0,
                loyalists: 0.0,
                dormant: 0.0,
            },
            ..Default::default()
        };
        let errors = config.validate();
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.field == "archetype_weights"));
    }

    #[test]
    fn campaign_variant_validate_accepts_valid() {
        let variant = CampaignVariant {
            variant_id: "test".into(),
            spend_budget: 1000.0,
            ..Default::default()
        };
        let errors = variant.validate();
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn campaign_variant_validate_rejects_empty_id() {
        let variant = CampaignVariant {
            variant_id: "".into(),
            spend_budget: 1000.0,
            ..Default::default()
        };
        let errors = variant.validate();
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.field == "variant_id"));
    }

    #[test]
    fn market_config_validate_rejects_empty_variants() {
        let config = MarketSimulationConfig {
            variants: vec![],
            ..Default::default()
        };
        let errors = config.validate();
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.field == "variants"));
    }

    #[test]
    fn config_digest_deterministic() {
        let config = MarketSimulationConfig::default();
        let d1 = config_digest(&config);
        let d2 = config_digest(&config);
        assert_eq!(d1, d2);
    }

    #[test]
    fn config_digest_differs_with_different_seed() {
        let mut config = MarketSimulationConfig::default();
        config.population.seed = 123;
        let d1 = config_digest(&config);
        config.population.seed = 456;
        let d2 = config_digest(&config);
        assert_ne!(d1, d2);
    }

    #[test]
    fn buyer_outcome_from_state_active() {
        let state = BuyerState {
            buyer_id: 1,
            archetype: BuyerArchetype::HighIntent,
            aware: true,
            considering: true,
            signup_t: 5,
            activated_t: 8,
            churned_t: -1,
            referral_count: 2,
            exposures: vec![],
        };
        let outcome = BuyerOutcome::from_state(&state, 500.0);
        assert!(outcome.reached_signup);
        assert!(outcome.reached_activation);
        assert!(!outcome.churned);
        assert_eq!(outcome.referral_count, 2);
        assert!(outcome.lifetime_value_cents > 0.0);
    }

    #[test]
    fn buyer_outcome_from_state_churned() {
        let state = BuyerState {
            buyer_id: 2,
            archetype: BuyerArchetype::Dormant,
            aware: false,
            considering: false,
            signup_t: -1,
            activated_t: -1,
            churned_t: -1,
            referral_count: 0,
            exposures: vec![],
        };
        let outcome = BuyerOutcome::from_state(&state, 500.0);
        assert!(!outcome.reached_signup);
        assert!(!outcome.reached_activation);
        assert!(!outcome.churned);
        assert_eq!(outcome.lifetime_value_cents, 0.0);
    }
}
