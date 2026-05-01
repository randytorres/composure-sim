use std::collections::{BTreeMap, BTreeSet};

use composure_population::{
    blueprint::{
        Budget, Channel, ChannelPreference, Objection, ObjectionType, SegmentStage,
        TraitDistributionConfig,
    },
    Buyer, PopulationConfig, PopulationGenerator, SegmentBlueprint as PopulationSegmentBlueprint,
};
use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptTestRequest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_seed_base")]
    pub seed_base: u64,
    #[serde(default)]
    pub population: ConceptPopulationConfig,
    #[serde(default)]
    pub segments: Vec<ConceptSegment>,
    #[serde(default)]
    pub variants: Vec<ConceptVariant>,
    #[serde(default)]
    pub scenario: ConceptScenario,
    #[serde(default)]
    pub observed_outcomes: Vec<ConceptObservedOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptPopulationConfig {
    #[serde(default = "default_population_size")]
    pub target_count: usize,
    #[serde(default = "default_sample_size")]
    pub sample_size: usize,
}

impl Default for ConceptPopulationConfig {
    fn default() -> Self {
        Self {
            target_count: default_population_size(),
            sample_size: default_sample_size(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptSegment {
    pub id: String,
    pub name: String,
    #[serde(alias = "share_prior", default = "default_share_weight")]
    pub share_weight: f64,
    #[serde(default)]
    pub traits: BTreeMap<String, f64>,
    #[serde(default)]
    pub channels: Vec<String>,
    #[serde(default)]
    pub objections: Vec<String>,
    #[serde(default)]
    pub target_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptVariant {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(alias = "best_for_segments", default)]
    pub target_segments: Vec<String>,
    #[serde(alias = "channel_fit", default)]
    pub channels: Vec<String>,
    #[serde(default)]
    pub trait_weights: BTreeMap<String, f64>,
    #[serde(default)]
    pub strengths: Vec<String>,
    #[serde(default)]
    pub risks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptScenario {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub goal: Option<String>,
    #[serde(default)]
    pub decision: Option<String>,
    #[serde(default)]
    pub channels: Vec<String>,
    #[serde(default = "default_time_steps")]
    pub time_steps: usize,
    #[serde(default)]
    pub success_metrics: Vec<String>,
    #[serde(default, alias = "sequence")]
    pub touchpoints: Vec<ConceptTouchpoint>,
}

impl Default for ConceptScenario {
    fn default() -> Self {
        Self {
            id: "default".into(),
            name: "Default Scenario".into(),
            goal: None,
            decision: None,
            channels: Vec::new(),
            time_steps: default_time_steps(),
            success_metrics: Vec::new(),
            touchpoints: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptTouchpoint {
    #[serde(default)]
    pub id: Option<String>,
    pub label: String,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub focus: ConceptTouchpointFocus,
    #[serde(default = "default_touchpoint_intensity")]
    pub intensity: f64,
    #[serde(default)]
    pub target_segments: Vec<String>,
    #[serde(default, alias = "variant_ids")]
    pub variants: Vec<String>,
    #[serde(default)]
    pub trait_weights: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConceptTouchpointFocus {
    Awareness,
    Resonance,
    Proof,
    ObjectionHandling,
    Conversion,
    Retention,
    Referral,
    Stressor,
}

impl Default for ConceptTouchpointFocus {
    fn default() -> Self {
        Self::Awareness
    }
}

impl ConceptTouchpointFocus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Awareness => "awareness",
            Self::Resonance => "resonance",
            Self::Proof => "proof",
            Self::ObjectionHandling => "objection_handling",
            Self::Conversion => "conversion",
            Self::Retention => "retention",
            Self::Referral => "referral",
            Self::Stressor => "stressor",
        }
    }
}

impl std::fmt::Display for ConceptTouchpointFocus {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptObservedOutcome {
    pub variant_id: String,
    #[serde(default, alias = "step_id")]
    pub touchpoint_id: Option<String>,
    #[serde(default)]
    pub segment_id: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub sample_size: Option<u32>,
    #[serde(default)]
    pub click_rate: Option<f64>,
    #[serde(default)]
    pub signup_rate: Option<f64>,
    #[serde(default)]
    pub activation_rate: Option<f64>,
    #[serde(default)]
    pub retention_rate: Option<f64>,
    #[serde(default)]
    pub referral_rate: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptTestResult {
    pub concept_test_id: String,
    pub name: String,
    pub scenario: ConceptScenario,
    pub total_population: usize,
    pub seed_base: u64,
    pub recommended_variant_id: Option<String>,
    #[serde(default)]
    pub top_ranked_variant_id: Option<String>,
    #[serde(default)]
    pub recommendation_status: ConceptRecommendationStatus,
    pub ranked_variants: Vec<ConceptVariantResult>,
    pub segment_summaries: Vec<ConceptSegmentSummary>,
    pub sampled_individuals: Vec<ConceptIndividualSample>,
    pub calibration_summary: Vec<ConceptCalibrationSummary>,
    #[serde(default)]
    pub touchpoint_calibration_summary: Vec<ConceptCalibrationSummary>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConceptRecommendationStatus {
    /// No usable observed outcomes were attached. The leaderboard is directional only;
    /// no variant is being recommended for production use.
    #[default]
    Uncalibrated,
    /// Top-ranked variant's score 95% CI overlaps with the runner-up's. Treat as tied.
    TiedWithinNoise,
    /// Calibrated against observed outcomes and top-2 score CIs do not overlap.
    CalibratedRecommended,
}

#[derive(Debug, Clone)]
pub struct ConceptTestComparisonInput {
    pub source: String,
    pub result: ConceptTestResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptTestComparisonReport {
    pub comparison_id: String,
    pub compared_tests: Vec<String>,
    pub recommendation: Vec<String>,
    pub repeated_winner_patterns: Vec<String>,
    pub tests: Vec<ConceptTestComparisonEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptTestMatrixRequest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub base_request: ConceptTestRequest,
    #[serde(default)]
    pub cases: Vec<ConceptTestMatrixCase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptTestMatrixCase {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub preset: Option<ConceptScenarioPreset>,
    #[serde(default)]
    pub scenario: Option<ConceptScenario>,
    #[serde(default)]
    pub seed_base: Option<u64>,
    #[serde(default)]
    pub population: Option<ConceptPopulationConfig>,
    #[serde(default)]
    pub segments: Vec<ConceptSegment>,
    #[serde(default)]
    pub observed_outcomes: Vec<ConceptObservedOutcome>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConceptScenarioPreset {
    CheapAcquisition,
    TrustCollapse,
    RetentionLoop,
    ReferralLoop,
    PricingPressure,
    OnboardingFriction,
    ProcurementSkepticism,
    GameplayFatigue,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConceptTestMatrixDraftRequest {
    pub prompt: String,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub product_name: Option<String>,
    #[serde(default)]
    pub domain: Option<ConceptDraftDomain>,
    #[serde(default)]
    pub population: Option<ConceptPopulationConfig>,
    #[serde(default)]
    pub case_count: Option<usize>,
    #[serde(default)]
    pub seed_base: Option<u64>,
    #[serde(default)]
    pub allowed_presets: Vec<ConceptScenarioPreset>,
    #[serde(default)]
    pub segments: Vec<ConceptSegment>,
    #[serde(default)]
    pub variants: Vec<ConceptVariant>,
    #[serde(default)]
    pub observed_outcomes: Vec<ConceptObservedOutcome>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConceptDraftDomain {
    Business,
    ConsumerProduct,
    EnterpriseSaas,
    Gameplay,
    DefenseProcurement,
    CivicPolicy,
    Creative,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptTestMatrixResult {
    pub matrix_id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub cases: Vec<ConceptTestMatrixCaseResult>,
    pub variant_rollups: Vec<ConceptTestMatrixVariantRollup>,
    pub recommendation: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptTestMatrixCaseResult {
    pub case_id: String,
    pub case_name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub scenario_id: String,
    pub scenario_name: String,
    pub top_variant_id: String,
    pub top_variant_score: u32,
    #[serde(default)]
    pub runner_up_variant_id: Option<String>,
    #[serde(default)]
    pub score_gap_vs_runner_up: Option<i32>,
    pub calibration_status: ConceptCalibrationStatus,
    pub result: ConceptTestResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptTestMatrixVariantRollup {
    pub variant_id: String,
    pub appearances: usize,
    pub wins: usize,
    pub average_score: u32,
    pub min_score: u32,
    pub max_score: u32,
    pub average_signup_rate: f64,
    pub average_retention_rate: f64,
    pub strong_cases: Vec<String>,
    pub weak_cases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptTestComparisonEntry {
    pub source: String,
    pub concept_test_id: String,
    pub name: String,
    pub scenario_name: String,
    pub total_population: usize,
    #[serde(default)]
    pub recommended_variant_id: Option<String>,
    pub top_variant_id: String,
    pub top_variant_score: u32,
    #[serde(default)]
    pub runner_up_variant_id: Option<String>,
    #[serde(default)]
    pub runner_up_variant_score: Option<u32>,
    #[serde(default)]
    pub score_gap_vs_runner_up: Option<i32>,
    pub top_variant_funnel: ConceptFunnelMetrics,
    #[serde(default)]
    pub metric_deltas: Vec<ConceptMetricDelta>,
    #[serde(default)]
    pub segment_winners: Vec<ConceptSegmentWinner>,
    #[serde(default)]
    pub calibration_status: ConceptCalibrationStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConceptCalibrationStatus {
    Uncalibrated,
    CalibratedSignal,
    ObservedWithoutRates,
}

impl Default for ConceptCalibrationStatus {
    fn default() -> Self {
        Self::Uncalibrated
    }
}

impl std::fmt::Display for ConceptCalibrationStatus {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Uncalibrated => "uncalibrated",
            Self::CalibratedSignal => "calibrated_signal",
            Self::ObservedWithoutRates => "observed_without_rates",
        };
        formatter.write_str(value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptMetricDelta {
    pub label: String,
    pub value: f64,
    pub delta_vs_compare_average: f64,
    pub delta_vs_compare_leader: f64,
    pub compare_set_rank: usize,
    pub compare_set_size: usize,
    pub leading_tests: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptSegmentWinner {
    pub segment_id: String,
    pub segment_name: String,
    pub best_variant_id: String,
    pub best_score: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptVariantResult {
    pub variant_id: String,
    pub variant_name: String,
    pub overall_score: u32,
    #[serde(default)]
    pub score_interval: ConceptScoreInterval,
    pub funnel: ConceptFunnelMetrics,
    #[serde(default)]
    pub touchpoint_results: Vec<ConceptTouchpointResult>,
    pub strongest_segments: Vec<String>,
    pub weakest_segments: Vec<String>,
    pub segment_results: Vec<ConceptSegmentResult>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConceptScoreInterval {
    /// Mean score, 0-100.
    pub mean: u32,
    /// 95% CI lower bound, 0-100. Equals `mean` when sample is too small.
    pub lower: u32,
    /// 95% CI upper bound, 0-100. Equals `mean` when sample is too small.
    pub upper: u32,
    /// Population standard deviation of buyer-level scores (unitless 0-1 scale).
    pub stddev: f64,
    /// Number of buyer scores aggregated into this interval.
    pub sample_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptTouchpointResult {
    pub index: usize,
    pub touchpoint_id: String,
    pub label: String,
    pub focus: ConceptTouchpointFocus,
    #[serde(default)]
    pub channel: Option<String>,
    pub buyers: usize,
    pub score: u32,
    #[serde(default)]
    pub lift_vs_previous_score: Option<i32>,
    pub funnel: ConceptFunnelMetrics,
    pub trait_fit_score: u32,
    pub channel_fit_score: u32,
    pub trust_score: u32,
    pub objection_pressure: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptSegmentResult {
    pub segment_id: String,
    pub segment_name: String,
    pub buyers: usize,
    pub score: u32,
    pub funnel: ConceptFunnelMetrics,
    pub trait_fit_score: u32,
    pub channel_fit_score: u32,
    pub trust_score: u32,
    pub objection_pressure: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptSegmentSummary {
    pub segment_id: String,
    pub segment_name: String,
    pub buyers_simulated: usize,
    pub best_variant_id: String,
    pub best_score: u32,
    #[serde(default)]
    pub runner_up_variant_id: Option<String>,
    #[serde(default)]
    pub runner_up_score: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConceptFunnelMetrics {
    pub buyers: usize,
    pub clicks: usize,
    pub signups: usize,
    pub activations: usize,
    pub retained: usize,
    pub referrals: usize,
    pub click_rate: f64,
    pub signup_rate: f64,
    pub activation_rate: f64,
    pub retention_rate: f64,
    pub referral_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptIndividualSample {
    pub individual_id: String,
    pub segment_id: String,
    pub top_variant_id: String,
    pub top_variant_score: u32,
    #[serde(default)]
    pub runner_up_variant_id: Option<String>,
    #[serde(default)]
    pub runner_up_score: Option<u32>,
    pub click_probability: u32,
    pub signup_probability: u32,
    pub activation_probability: u32,
    pub retention_probability: u32,
    pub referral_probability: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptCalibrationSummary {
    pub variant_id: String,
    #[serde(default)]
    pub touchpoint_id: Option<String>,
    pub observed_records: usize,
    pub usable_observed_records: usize,
    #[serde(default)]
    pub observed_sample_size: Option<u32>,
    #[serde(default)]
    pub compared_metrics: Vec<String>,
    #[serde(default)]
    pub click_gap: Option<f64>,
    #[serde(default)]
    pub signup_gap: Option<f64>,
    #[serde(default)]
    pub activation_gap: Option<f64>,
    #[serde(default)]
    pub retention_gap: Option<f64>,
    #[serde(default)]
    pub referral_gap: Option<f64>,
    pub note: String,
}

#[derive(Debug, Clone, Default)]
struct ConceptAccumulator {
    buyers: usize,
    score_sum: f64,
    score_sum_sq: f64,
    trait_fit_sum: f64,
    channel_fit_sum: f64,
    trust_sum: f64,
    objection_pressure_sum: f64,
    clicks: usize,
    signups: usize,
    activations: usize,
    retained: usize,
    referrals: usize,
}

#[derive(Debug, Clone)]
struct ScoredVariant {
    variant_id: String,
    score: f64,
    trait_fit: f64,
    channel_fit: f64,
    trust: f64,
    objection_pressure: f64,
    funnel: IndividualFunnel,
    touchpoints: Vec<ScoredTouchpoint>,
}

#[derive(Debug, Clone)]
struct ScoredTouchpoint {
    score: f64,
    trait_fit: f64,
    channel_fit: f64,
    trust: f64,
    objection_pressure: f64,
    funnel: IndividualFunnel,
}

#[derive(Debug, Clone, Copy)]
struct BuyerVariantComponents {
    segment_fit: f64,
    trait_fit: f64,
    channel_fit: f64,
    trust: f64,
    objection_pressure: f64,
    time_factor: f64,
}

#[derive(Debug, Clone)]
struct IndividualFunnel {
    click_probability: f64,
    signup_probability: f64,
    activation_probability: f64,
    retention_probability: f64,
    referral_probability: f64,
    clicked: bool,
    signed_up: bool,
    activated: bool,
    retained: bool,
    referred: bool,
}

pub fn simulate_concept_test(
    request: &ConceptTestRequest,
) -> Result<ConceptTestResult, ConceptTestError> {
    validate_request(request)?;

    let population_blueprints = request
        .segments
        .iter()
        .map(to_population_blueprint)
        .collect::<Vec<_>>();
    let population = PopulationGenerator::new(PopulationConfig {
        population_seed: request.seed_base,
        target_count: request.population.target_count,
        use_correlation: false,
        correlation_specs: BTreeMap::new(),
    })
    .generate(&population_blueprints)
    .map_err(ConceptTestError::Population)?;

    let segments_by_id = request
        .segments
        .iter()
        .map(|segment| (segment.id.as_str(), segment))
        .collect::<BTreeMap<_, _>>();
    let variants_by_id = request
        .variants
        .iter()
        .map(|variant| (variant.id.as_str(), variant))
        .collect::<BTreeMap<_, _>>();

    let mut variant_accumulators = request
        .variants
        .iter()
        .map(|variant| (variant.id.clone(), ConceptAccumulator::default()))
        .collect::<BTreeMap<_, _>>();
    let mut variant_touchpoint_accumulators = request
        .variants
        .iter()
        .map(|variant| {
            (
                variant.id.clone(),
                request
                    .scenario
                    .touchpoints
                    .iter()
                    .map(|_| ConceptAccumulator::default())
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut segment_variant_accumulators =
        BTreeMap::<String, BTreeMap<String, ConceptAccumulator>>::new();
    let mut sampled_individuals = Vec::new();
    let sample_stride = if request.population.sample_size == 0 {
        usize::MAX
    } else {
        (population.buyers.len() / request.population.sample_size.max(1)).max(1)
    };

    for (buyer_index, buyer) in population.buyers.iter().enumerate() {
        let segment = segments_by_id
            .get(buyer.segment_id.as_str())
            .ok_or_else(|| ConceptTestError::MissingGeneratedSegment(buyer.segment_id.clone()))?;

        let mut scored = request
            .variants
            .iter()
            .map(|variant| {
                score_buyer_against_variant(
                    buyer,
                    segment,
                    variant,
                    &request.scenario,
                    request.seed_base,
                )
            })
            .collect::<Vec<_>>();

        for score in &scored {
            if let Some(accumulator) = variant_accumulators.get_mut(&score.variant_id) {
                accumulator.add(score);
            }
            if let Some(touchpoint_accumulators) =
                variant_touchpoint_accumulators.get_mut(&score.variant_id)
            {
                for (index, touchpoint_score) in score.touchpoints.iter().enumerate() {
                    if let Some(accumulator) = touchpoint_accumulators.get_mut(index) {
                        accumulator.add_touchpoint(touchpoint_score);
                    }
                }
            }
            segment_variant_accumulators
                .entry(buyer.segment_id.clone())
                .or_default()
                .entry(score.variant_id.clone())
                .or_default()
                .add(score);
        }

        if request.population.sample_size > 0
            && sampled_individuals.len() < request.population.sample_size
            && buyer_index % sample_stride == 0
        {
            scored.sort_by(|left, right| {
                right
                    .score
                    .partial_cmp(&left.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| left.variant_id.cmp(&right.variant_id))
            });
            if let Some(top) = scored.first() {
                let runner_up = scored.get(1);
                sampled_individuals.push(ConceptIndividualSample {
                    individual_id: buyer.id.0.clone(),
                    segment_id: buyer.segment_id.clone(),
                    top_variant_id: top.variant_id.clone(),
                    top_variant_score: as_percent(top.score),
                    runner_up_variant_id: runner_up.map(|score| score.variant_id.clone()),
                    runner_up_score: runner_up.map(|score| as_percent(score.score)),
                    click_probability: as_percent(top.funnel.click_probability),
                    signup_probability: as_percent(top.funnel.signup_probability),
                    activation_probability: as_percent(top.funnel.activation_probability),
                    retention_probability: as_percent(top.funnel.retention_probability),
                    referral_probability: as_percent(top.funnel.referral_probability),
                });
            }
        }
    }

    let mut ranked_variants = request
        .variants
        .iter()
        .map(|variant| {
            let accumulator = variant_accumulators
                .get(&variant.id)
                .cloned()
                .unwrap_or_default();
            let mut segment_results = request
                .segments
                .iter()
                .filter_map(|segment| {
                    segment_variant_accumulators
                        .get(&segment.id)
                        .and_then(|by_variant| by_variant.get(&variant.id))
                        .map(|segment_accumulator| {
                            build_segment_result(segment, segment_accumulator)
                        })
                })
                .collect::<Vec<_>>();

            segment_results.sort_by(|left, right| {
                right
                    .score
                    .cmp(&left.score)
                    .then_with(|| left.segment_id.cmp(&right.segment_id))
            });

            let strongest_segments = segment_results
                .iter()
                .take(2)
                .map(|segment| segment.segment_id.clone())
                .collect::<Vec<_>>();
            let weakest_segments = segment_results
                .iter()
                .rev()
                .take(2)
                .map(|segment| segment.segment_id.clone())
                .collect::<Vec<_>>();

            ConceptVariantResult {
                variant_id: variant.id.clone(),
                variant_name: variant.name.clone(),
                overall_score: accumulator.average_score(),
                score_interval: accumulator.score_interval(),
                funnel: accumulator.funnel(),
                touchpoint_results: build_touchpoint_results(
                    &request.scenario.touchpoints,
                    variant_touchpoint_accumulators
                        .get(&variant.id)
                        .map(Vec::as_slice)
                        .unwrap_or(&[]),
                ),
                strongest_segments,
                weakest_segments,
                segment_results,
            }
        })
        .collect::<Vec<_>>();

    ranked_variants.sort_by(|left, right| {
        right
            .overall_score
            .cmp(&left.overall_score)
            .then_with(|| left.variant_id.cmp(&right.variant_id))
    });

    let segment_summaries = build_segment_summaries(
        &request.segments,
        &segment_variant_accumulators,
        &variants_by_id,
    );
    let top_ranked_variant_id = ranked_variants
        .first()
        .map(|variant| variant.variant_id.clone());
    let calibration_summary =
        build_calibration_summary(&request.observed_outcomes, &ranked_variants);
    let touchpoint_calibration_summary =
        build_touchpoint_calibration_summary(&request.observed_outcomes, &ranked_variants);
    let recommendation_status = compute_recommendation_status(
        &ranked_variants,
        &calibration_summary,
        &touchpoint_calibration_summary,
    );
    let recommended_variant_id = if matches!(
        recommendation_status,
        ConceptRecommendationStatus::CalibratedRecommended
    ) {
        top_ranked_variant_id.clone()
    } else {
        None
    };
    let notes = build_notes(
        request,
        &ranked_variants,
        &calibration_summary,
        &touchpoint_calibration_summary,
        &recommendation_status,
    );

    Ok(ConceptTestResult {
        concept_test_id: request.id.clone(),
        name: request.name.clone(),
        scenario: request.scenario.clone(),
        total_population: population.buyer_count,
        seed_base: request.seed_base,
        recommended_variant_id,
        top_ranked_variant_id,
        recommendation_status,
        ranked_variants,
        segment_summaries,
        sampled_individuals,
        calibration_summary,
        touchpoint_calibration_summary,
        notes,
    })
}

fn compute_recommendation_status(
    ranked_variants: &[ConceptVariantResult],
    calibration_summary: &[ConceptCalibrationSummary],
    touchpoint_calibration_summary: &[ConceptCalibrationSummary],
) -> ConceptRecommendationStatus {
    let has_usable_calibration = calibration_summary
        .iter()
        .chain(touchpoint_calibration_summary.iter())
        .any(|summary| summary.usable_observed_records > 0);
    if !has_usable_calibration {
        return ConceptRecommendationStatus::Uncalibrated;
    }
    match (ranked_variants.first(), ranked_variants.get(1)) {
        (Some(top), Some(runner_up))
            if top.score_interval.lower <= runner_up.score_interval.upper =>
        {
            ConceptRecommendationStatus::TiedWithinNoise
        }
        _ => ConceptRecommendationStatus::CalibratedRecommended,
    }
}

pub fn compare_concept_test_results(
    inputs: &[ConceptTestComparisonInput],
) -> Result<ConceptTestComparisonReport, ConceptTestCompareError> {
    if inputs.len() < 2 {
        return Err(ConceptTestCompareError::TooFewInputs);
    }

    let mut entries = inputs
        .iter()
        .map(build_comparison_entry)
        .collect::<Result<Vec<_>, _>>()?;
    fill_metric_deltas(&mut entries);
    entries.sort_by(|left, right| {
        right
            .top_variant_score
            .cmp(&left.top_variant_score)
            .then_with(|| {
                right
                    .top_variant_funnel
                    .signup_rate
                    .total_cmp(&left.top_variant_funnel.signup_rate)
            })
            .then_with(|| left.name.cmp(&right.name))
    });

    let recommendation = build_comparison_recommendation(&entries);
    let repeated_winner_patterns = build_repeated_winner_patterns(&entries);
    let compared_tests = inputs
        .iter()
        .map(|input| input.source.clone())
        .collect::<Vec<_>>();

    Ok(ConceptTestComparisonReport {
        comparison_id: format!("concept-compare-{}", inputs.len()),
        compared_tests,
        recommendation,
        repeated_winner_patterns,
        tests: entries,
    })
}

pub fn draft_concept_test_matrix(
    request: &ConceptTestMatrixDraftRequest,
) -> Result<ConceptTestMatrixRequest, ConceptTestMatrixDraftError> {
    let prompt = request.prompt.trim();
    if prompt.is_empty() {
        return Err(ConceptTestMatrixDraftError::EmptyPrompt);
    }
    if let Some(case_count) = request.case_count {
        if case_count == 0 {
            return Err(ConceptTestMatrixDraftError::InvalidCaseCount(case_count));
        }
    }

    let domain = request.domain.unwrap_or_else(|| infer_draft_domain(prompt));
    let title = request
        .name
        .clone()
        .or_else(|| request.product_name.clone())
        .unwrap_or_else(|| title_from_prompt(prompt));
    let id_stem = request
        .id
        .clone()
        .unwrap_or_else(|| slugify_non_empty(&title, "concept-draft"));
    let seed_base = request
        .seed_base
        .unwrap_or_else(|| stable_text_seed("concept-matrix-draft", prompt, &id_stem));
    let population = request
        .population
        .clone()
        .unwrap_or(ConceptPopulationConfig {
            target_count: 10_000,
            sample_size: default_sample_size(),
        });
    let channels = draft_channels(domain, prompt);
    let segments = if request.segments.is_empty() {
        draft_segments(domain)
    } else {
        request.segments.clone()
    };
    let variants = if request.variants.is_empty() {
        draft_variants(domain, prompt, &segments, &channels)
    } else {
        request.variants.clone()
    };

    let base_request = ConceptTestRequest {
        id: format!("{id_stem}-base"),
        name: format!("{title} Base Concept Test"),
        description: Some(prompt_excerpt(prompt, 320)),
        seed_base,
        population: population.clone(),
        segments,
        variants,
        scenario: ConceptScenario {
            id: "baseline".into(),
            name: "Baseline Decision Scenario".into(),
            goal: Some(draft_goal(domain).into()),
            decision: Some(draft_decision(domain).into()),
            channels: channels.clone(),
            time_steps: draft_time_steps(domain),
            success_metrics: draft_success_metrics(domain),
            touchpoints: draft_baseline_touchpoints(domain, &channels),
        },
        observed_outcomes: request.observed_outcomes.clone(),
    };

    let cases = draft_matrix_cases(
        domain,
        prompt,
        request.case_count,
        &request.allowed_presets,
        seed_base,
        &population,
    );

    let matrix = ConceptTestMatrixRequest {
        id: format!("{id_stem}-matrix"),
        name: format!("{title} Scenario Matrix"),
        description: request.description.clone().or_else(|| {
            Some(format!(
                "Drafted from brief: {}",
                prompt_excerpt(prompt, 220)
            ))
        }),
        base_request,
        cases,
    };

    validate_request(&matrix.base_request).map_err(ConceptTestMatrixDraftError::ConceptTest)?;
    validate_matrix_request(&matrix).map_err(ConceptTestMatrixDraftError::Matrix)?;
    Ok(matrix)
}

pub fn run_concept_test_matrix(
    request: &ConceptTestMatrixRequest,
) -> Result<ConceptTestMatrixResult, ConceptTestMatrixError> {
    validate_matrix_request(request)?;

    let mut cases = Vec::new();
    for case in &request.cases {
        let case_request = build_matrix_case_request(&request.base_request, case);
        let result = simulate_concept_test(&case_request).map_err(|source| {
            ConceptTestMatrixError::CaseSimulation {
                case_id: case.id.clone(),
                source,
            }
        })?;
        let top = result
            .ranked_variants
            .first()
            .ok_or_else(|| ConceptTestMatrixError::EmptyCaseResult(case.id.clone()))?;
        let runner_up = result.ranked_variants.get(1);

        cases.push(ConceptTestMatrixCaseResult {
            case_id: case.id.clone(),
            case_name: case.name.clone(),
            description: case.description.clone(),
            scenario_id: result.scenario.id.clone(),
            scenario_name: result.scenario.name.clone(),
            top_variant_id: top.variant_id.clone(),
            top_variant_score: top.overall_score,
            runner_up_variant_id: runner_up.map(|variant| variant.variant_id.clone()),
            score_gap_vs_runner_up: runner_up
                .map(|variant| top.overall_score as i32 - variant.overall_score as i32),
            calibration_status: calibration_status(&result),
            result,
        });
    }

    let variant_rollups = build_matrix_variant_rollups(&cases);
    let recommendation = build_matrix_recommendation(&variant_rollups, &cases);
    let notes = build_matrix_notes(request, &cases, &variant_rollups);

    Ok(ConceptTestMatrixResult {
        matrix_id: request.id.clone(),
        name: request.name.clone(),
        description: request.description.clone(),
        cases,
        variant_rollups,
        recommendation,
        notes,
    })
}

fn validate_matrix_request(
    request: &ConceptTestMatrixRequest,
) -> Result<(), ConceptTestMatrixError> {
    validate_non_empty(&request.id, ConceptTestError::EmptyId)
        .map_err(|_| ConceptTestMatrixError::EmptyId)?;
    validate_non_empty(&request.name, ConceptTestError::EmptyName)
        .map_err(|_| ConceptTestMatrixError::EmptyName)?;
    if request.cases.is_empty() {
        return Err(ConceptTestMatrixError::NoCases);
    }

    let mut case_ids = BTreeSet::new();
    for case in &request.cases {
        if case.id.trim().is_empty() {
            return Err(ConceptTestMatrixError::EmptyCaseId);
        }
        if case.name.trim().is_empty() {
            return Err(ConceptTestMatrixError::EmptyCaseName);
        }
        if !case_ids.insert(case.id.clone()) {
            return Err(ConceptTestMatrixError::DuplicateCaseId(case.id.clone()));
        }
        if case.scenario.is_none() && case.preset.is_none() {
            return Err(ConceptTestMatrixError::MissingCaseScenario(case.id.clone()));
        }
    }

    Ok(())
}

fn build_matrix_case_request(
    base_request: &ConceptTestRequest,
    case: &ConceptTestMatrixCase,
) -> ConceptTestRequest {
    let mut request = base_request.clone();
    request.id = format!("{}::{}", base_request.id, case.id);
    request.name = format!("{} / {}", base_request.name, case.name);
    request.scenario = resolve_matrix_case_scenario(base_request, case);
    request.observed_outcomes =
        filter_observed_outcomes_for_scenario(&request.observed_outcomes, &request.scenario);
    if let Some(seed_base) = case.seed_base {
        request.seed_base = seed_base;
    }
    if let Some(population) = &case.population {
        request.population = population.clone();
    }
    if !case.segments.is_empty() {
        request.segments = case.segments.clone();
    }
    if !case.observed_outcomes.is_empty() {
        request.observed_outcomes = case.observed_outcomes.clone();
    }
    request
}

fn filter_observed_outcomes_for_scenario(
    observed_outcomes: &[ConceptObservedOutcome],
    scenario: &ConceptScenario,
) -> Vec<ConceptObservedOutcome> {
    let touchpoint_ids = scenario
        .touchpoints
        .iter()
        .enumerate()
        .map(|(index, touchpoint)| touchpoint_key(index, touchpoint))
        .collect::<BTreeSet<_>>();

    observed_outcomes
        .iter()
        .filter(|outcome| match outcome.touchpoint_id.as_deref() {
            None => true,
            Some(touchpoint_id) => touchpoint_ids.contains(touchpoint_id),
        })
        .cloned()
        .collect()
}

fn resolve_matrix_case_scenario(
    base_request: &ConceptTestRequest,
    case: &ConceptTestMatrixCase,
) -> ConceptScenario {
    case.scenario.clone().unwrap_or_else(|| {
        scenario_from_preset(
            case.preset
                .expect("matrix case preset is validated before scenario resolution"),
            base_request,
            case,
        )
    })
}

fn scenario_from_preset(
    preset: ConceptScenarioPreset,
    base_request: &ConceptTestRequest,
    case: &ConceptTestMatrixCase,
) -> ConceptScenario {
    let metrics = preset_success_metrics(preset, &base_request.scenario);
    let time_steps = match preset {
        ConceptScenarioPreset::CheapAcquisition => 6,
        ConceptScenarioPreset::TrustCollapse => 8,
        ConceptScenarioPreset::RetentionLoop => 10,
        ConceptScenarioPreset::ReferralLoop => 8,
        ConceptScenarioPreset::PricingPressure => 7,
        ConceptScenarioPreset::OnboardingFriction => 8,
        ConceptScenarioPreset::ProcurementSkepticism => 10,
        ConceptScenarioPreset::GameplayFatigue => 10,
    };

    ConceptScenario {
        id: case.id.clone(),
        name: case.name.clone(),
        goal: Some(preset_goal(preset).into()),
        decision: Some(preset_decision(preset).into()),
        channels: preset_channels(preset, &base_request.scenario),
        time_steps,
        success_metrics: metrics,
        touchpoints: preset_touchpoints(preset),
    }
}

fn preset_success_metrics(
    preset: ConceptScenarioPreset,
    base_scenario: &ConceptScenario,
) -> Vec<String> {
    let metrics = match preset {
        ConceptScenarioPreset::CheapAcquisition => vec!["click_rate", "signup_rate"],
        ConceptScenarioPreset::TrustCollapse => vec!["signup_rate", "retention_rate"],
        ConceptScenarioPreset::RetentionLoop => vec!["activation_rate", "retention_rate"],
        ConceptScenarioPreset::ReferralLoop => vec!["signup_rate", "referral_rate"],
        ConceptScenarioPreset::PricingPressure => vec!["signup_rate", "activation_rate"],
        ConceptScenarioPreset::OnboardingFriction => vec!["activation_rate", "retention_rate"],
        ConceptScenarioPreset::ProcurementSkepticism => vec!["activation_rate", "retention_rate"],
        ConceptScenarioPreset::GameplayFatigue => vec!["retention_rate", "referral_rate"],
    };
    if metrics.is_empty() && !base_scenario.success_metrics.is_empty() {
        base_scenario.success_metrics.clone()
    } else {
        metrics.into_iter().map(str::to_string).collect()
    }
}

fn preset_channels(preset: ConceptScenarioPreset, base_scenario: &ConceptScenario) -> Vec<String> {
    let channels = match preset {
        ConceptScenarioPreset::CheapAcquisition => vec!["reddit", "landing_page"],
        ConceptScenarioPreset::TrustCollapse => vec!["reddit", "newsletter"],
        ConceptScenarioPreset::RetentionLoop => vec!["email", "in_app", "community"],
        ConceptScenarioPreset::ReferralLoop => vec!["community", "reddit", "newsletter"],
        ConceptScenarioPreset::PricingPressure => vec!["landing_page", "sales_call"],
        ConceptScenarioPreset::OnboardingFriction => vec!["landing_page", "in_app", "email"],
        ConceptScenarioPreset::ProcurementSkepticism => {
            vec!["technical_brief", "demo", "procurement_review"]
        }
        ConceptScenarioPreset::GameplayFatigue => vec!["gameplay", "community", "in_game"],
    };
    if channels.is_empty() && !base_scenario.channels.is_empty() {
        base_scenario.channels.clone()
    } else {
        channels.into_iter().map(str::to_string).collect()
    }
}

fn preset_goal(preset: ConceptScenarioPreset) -> &'static str {
    match preset {
        ConceptScenarioPreset::CheapAcquisition => "Find the strongest low-cost acquisition wedge.",
        ConceptScenarioPreset::TrustCollapse => "Test which variant survives a sudden trust loss.",
        ConceptScenarioPreset::RetentionLoop => {
            "Find the variant most likely to sustain repeat use."
        }
        ConceptScenarioPreset::ReferralLoop => {
            "Find the variant most likely to create second-order spread."
        }
        ConceptScenarioPreset::PricingPressure => {
            "Test which variant survives price and budget scrutiny."
        }
        ConceptScenarioPreset::OnboardingFriction => {
            "Find the variant that keeps momentum through setup friction."
        }
        ConceptScenarioPreset::ProcurementSkepticism => {
            "Test which variant survives formal buyer skepticism and review."
        }
        ConceptScenarioPreset::GameplayFatigue => {
            "Find the variant that keeps engagement after repeated play."
        }
    }
}

fn preset_decision(preset: ConceptScenarioPreset) -> &'static str {
    match preset {
        ConceptScenarioPreset::CheapAcquisition => "Pick a waitlist or ad-test control.",
        ConceptScenarioPreset::TrustCollapse => {
            "Pick the variant to test under privacy or credibility pressure."
        }
        ConceptScenarioPreset::RetentionLoop => "Pick the onboarding and follow-up control.",
        ConceptScenarioPreset::ReferralLoop => "Pick the community or referral-loop control.",
        ConceptScenarioPreset::PricingPressure => {
            "Pick the offer to test against budget objections."
        }
        ConceptScenarioPreset::OnboardingFriction => {
            "Pick the variant to test in a high-friction onboarding flow."
        }
        ConceptScenarioPreset::ProcurementSkepticism => {
            "Pick the variant to test with technical and procurement evaluators."
        }
        ConceptScenarioPreset::GameplayFatigue => {
            "Pick the loop to test for repeated-session retention."
        }
    }
}

fn preset_touchpoints(preset: ConceptScenarioPreset) -> Vec<ConceptTouchpoint> {
    match preset {
        ConceptScenarioPreset::CheapAcquisition => vec![
            preset_touchpoint(
                "awareness_hook",
                "Awareness Hook",
                "reddit",
                ConceptTouchpointFocus::Awareness,
                1.15,
            ),
            preset_touchpoint(
                "proof_page",
                "Proof Page",
                "landing_page",
                ConceptTouchpointFocus::Proof,
                1.10,
            ),
            preset_touchpoint(
                "conversion_cta",
                "Conversion CTA",
                "landing_page",
                ConceptTouchpointFocus::Conversion,
                0.95,
            ),
        ],
        ConceptScenarioPreset::TrustCollapse => vec![
            preset_touchpoint(
                "trust_stressor",
                "Trust Stressor",
                "reddit",
                ConceptTouchpointFocus::Stressor,
                1.25,
            ),
            preset_touchpoint(
                "objection_recovery",
                "Objection Recovery",
                "newsletter",
                ConceptTouchpointFocus::ObjectionHandling,
                1.20,
            ),
            preset_touchpoint(
                "retention_check",
                "Retention Check",
                "newsletter",
                ConceptTouchpointFocus::Retention,
                0.95,
            ),
        ],
        ConceptScenarioPreset::RetentionLoop => vec![
            preset_touchpoint(
                "first_value",
                "First Value Moment",
                "in_app",
                ConceptTouchpointFocus::Conversion,
                1.00,
            ),
            preset_touchpoint(
                "habit_followup",
                "Habit Follow-up",
                "email",
                ConceptTouchpointFocus::Retention,
                1.15,
            ),
            preset_touchpoint(
                "community_reinforcement",
                "Community Reinforcement",
                "community",
                ConceptTouchpointFocus::Resonance,
                0.90,
            ),
        ],
        ConceptScenarioPreset::ReferralLoop => vec![
            preset_touchpoint(
                "community_hook",
                "Community Hook",
                "community",
                ConceptTouchpointFocus::Awareness,
                1.05,
            ),
            preset_touchpoint(
                "peer_proof",
                "Peer Proof",
                "reddit",
                ConceptTouchpointFocus::Proof,
                1.10,
            ),
            preset_touchpoint(
                "referral_prompt",
                "Referral Prompt",
                "community",
                ConceptTouchpointFocus::Referral,
                1.00,
            ),
        ],
        ConceptScenarioPreset::PricingPressure => vec![
            preset_touchpoint(
                "price_stressor",
                "Price Stressor",
                "landing_page",
                ConceptTouchpointFocus::Stressor,
                1.10,
            ),
            preset_touchpoint(
                "value_proof",
                "Value Proof",
                "sales_call",
                ConceptTouchpointFocus::Proof,
                1.10,
            ),
            preset_touchpoint(
                "objection_handling",
                "Budget Objection Handling",
                "sales_call",
                ConceptTouchpointFocus::ObjectionHandling,
                1.05,
            ),
        ],
        ConceptScenarioPreset::OnboardingFriction => vec![
            preset_touchpoint(
                "signup_cta",
                "Signup CTA",
                "landing_page",
                ConceptTouchpointFocus::Conversion,
                0.95,
            ),
            preset_touchpoint(
                "setup_stressor",
                "Setup Friction",
                "in_app",
                ConceptTouchpointFocus::Stressor,
                1.20,
            ),
            preset_touchpoint(
                "activation_recovery",
                "Activation Recovery",
                "email",
                ConceptTouchpointFocus::ObjectionHandling,
                1.10,
            ),
        ],
        ConceptScenarioPreset::ProcurementSkepticism => vec![
            preset_touchpoint(
                "technical_review",
                "Technical Review",
                "technical_brief",
                ConceptTouchpointFocus::Proof,
                1.10,
            ),
            preset_touchpoint(
                "procurement_stressor",
                "Procurement Skepticism",
                "procurement_review",
                ConceptTouchpointFocus::Stressor,
                1.20,
            ),
            preset_touchpoint(
                "demo_recovery",
                "Demo Recovery",
                "demo",
                ConceptTouchpointFocus::ObjectionHandling,
                1.15,
            ),
            preset_touchpoint(
                "retention_risk_review",
                "Retention Risk Review",
                "procurement_review",
                ConceptTouchpointFocus::Retention,
                0.90,
            ),
        ],
        ConceptScenarioPreset::GameplayFatigue => vec![
            preset_touchpoint(
                "first_session",
                "First Session Hook",
                "gameplay",
                ConceptTouchpointFocus::Awareness,
                1.10,
            ),
            preset_touchpoint(
                "fatigue_stressor",
                "Repeated Play Fatigue",
                "gameplay",
                ConceptTouchpointFocus::Stressor,
                1.20,
            ),
            preset_touchpoint(
                "mastery_recovery",
                "Mastery Recovery",
                "in_game",
                ConceptTouchpointFocus::Retention,
                1.05,
            ),
            preset_touchpoint(
                "community_share",
                "Community Share",
                "community",
                ConceptTouchpointFocus::Referral,
                0.95,
            ),
        ],
    }
}

fn preset_touchpoint(
    id: &str,
    label: &str,
    channel: &str,
    focus: ConceptTouchpointFocus,
    intensity: f64,
) -> ConceptTouchpoint {
    ConceptTouchpoint {
        id: Some(id.into()),
        label: label.into(),
        channel: Some(channel.into()),
        focus,
        intensity,
        target_segments: Vec::new(),
        variants: Vec::new(),
        trait_weights: BTreeMap::new(),
    }
}

fn build_matrix_variant_rollups(
    cases: &[ConceptTestMatrixCaseResult],
) -> Vec<ConceptTestMatrixVariantRollup> {
    let mut by_variant =
        BTreeMap::<String, Vec<(&ConceptTestMatrixCaseResult, &ConceptVariantResult)>>::new();
    for case in cases {
        for variant in &case.result.ranked_variants {
            by_variant
                .entry(variant.variant_id.clone())
                .or_default()
                .push((case, variant));
        }
    }

    let mut rollups = by_variant
        .into_iter()
        .map(|(variant_id, entries)| {
            let appearances = entries.len();
            let wins = entries
                .iter()
                .filter(|(case, _)| case.top_variant_id == variant_id)
                .count();
            let score_sum = entries
                .iter()
                .map(|(_, variant)| variant.overall_score as usize)
                .sum::<usize>();
            let average_score = (score_sum as f64 / appearances as f64).round() as u32;
            let min_score = entries
                .iter()
                .map(|(_, variant)| variant.overall_score)
                .min()
                .unwrap_or_default();
            let max_score = entries
                .iter()
                .map(|(_, variant)| variant.overall_score)
                .max()
                .unwrap_or_default();
            let average_signup_rate = entries
                .iter()
                .map(|(_, variant)| variant.funnel.signup_rate)
                .sum::<f64>()
                / appearances as f64;
            let average_retention_rate = entries
                .iter()
                .map(|(_, variant)| variant.funnel.retention_rate)
                .sum::<f64>()
                / appearances as f64;

            let mut ranked_cases = entries
                .iter()
                .map(|(case, variant)| (case.case_id.clone(), variant.overall_score))
                .collect::<Vec<_>>();
            ranked_cases
                .sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
            let strong_cases = ranked_cases
                .iter()
                .take(2)
                .map(|(case_id, _)| case_id.clone())
                .collect::<Vec<_>>();
            let weak_cases = ranked_cases
                .iter()
                .rev()
                .take(2)
                .map(|(case_id, _)| case_id.clone())
                .collect::<Vec<_>>();

            ConceptTestMatrixVariantRollup {
                variant_id,
                appearances,
                wins,
                average_score,
                min_score,
                max_score,
                average_signup_rate,
                average_retention_rate,
                strong_cases,
                weak_cases,
            }
        })
        .collect::<Vec<_>>();

    rollups.sort_by(|left, right| {
        right
            .wins
            .cmp(&left.wins)
            .then_with(|| right.average_score.cmp(&left.average_score))
            .then_with(|| right.min_score.cmp(&left.min_score))
            .then_with(|| left.variant_id.cmp(&right.variant_id))
    });
    rollups
}

fn build_matrix_recommendation(
    rollups: &[ConceptTestMatrixVariantRollup],
    cases: &[ConceptTestMatrixCaseResult],
) -> Vec<String> {
    let mut recommendation = Vec::new();
    if let Some(best) = rollups.first() {
        recommendation.push(format!(
            "`{}` is the most robust variant across the matrix: `{}` wins over `{}` cases, average score `{}`, floor score `{}`.",
            best.variant_id,
            best.wins,
            cases.len(),
            best.average_score,
            best.min_score
        ));
        if !best.weak_cases.is_empty() {
            recommendation.push(format!(
                "Stress-test `{}` next in: {}.",
                best.variant_id,
                best.weak_cases.join(", ")
            ));
        }
    }
    if let Some(case) = cases
        .iter()
        .filter(|case| case.score_gap_vs_runner_up.is_some())
        .min_by_key(|case| case.score_gap_vs_runner_up.unwrap_or(i32::MAX))
    {
        recommendation.push(format!(
            "`{}` is the least decisive case; use it as the next real-world learning test.",
            case.case_name
        ));
    }
    recommendation
}

fn build_matrix_notes(
    request: &ConceptTestMatrixRequest,
    cases: &[ConceptTestMatrixCaseResult],
    rollups: &[ConceptTestMatrixVariantRollup],
) -> Vec<String> {
    let mut notes = Vec::new();
    notes.push(format!(
        "Matrix ran `{}` cases from base concept test `{}`.",
        cases.len(),
        request.base_request.id
    ));
    if cases
        .iter()
        .all(|case| case.calibration_status == ConceptCalibrationStatus::Uncalibrated)
    {
        notes.push("No matrix cases included usable observed outcomes, so use this for prioritization rather than forecasting.".into());
    }
    if rollups.len() > 1 {
        notes.push(format!(
            "Compared `{}` variants for robustness across scenario assumptions.",
            rollups.len()
        ));
    }
    notes
}

fn infer_draft_domain(prompt: &str) -> ConceptDraftDomain {
    let text = prompt.to_ascii_lowercase();
    if contains_any(
        &text,
        &[
            "gameplay",
            "game loop",
            "player",
            "quest",
            "session",
            "matchmaking",
            "retention mechanic",
        ],
    ) {
        ConceptDraftDomain::Gameplay
    } else if contains_any(
        &text,
        &[
            "military",
            "defense",
            "warfighter",
            "dod",
            "procurement",
            "tactical",
            "mission",
            "operator kit",
        ],
    ) {
        ConceptDraftDomain::DefenseProcurement
    } else if contains_any(
        &text,
        &[
            "enterprise",
            "saas",
            "sales-led",
            "b2b",
            "security review",
            "admin",
        ],
    ) {
        ConceptDraftDomain::EnterpriseSaas
    } else if contains_any(
        &text,
        &[
            "policy",
            "public opinion",
            "campaign",
            "regulation",
            "constituent",
        ],
    ) {
        ConceptDraftDomain::CivicPolicy
    } else if contains_any(
        &text,
        &[
            "story",
            "film",
            "character",
            "audience reaction",
            "creative",
        ],
    ) {
        ConceptDraftDomain::Creative
    } else if contains_any(
        &text,
        &["consumer", "d2c", "retail", "mobile app", "subscription"],
    ) {
        ConceptDraftDomain::ConsumerProduct
    } else {
        ConceptDraftDomain::Business
    }
}

fn title_from_prompt(prompt: &str) -> String {
    let candidate = prompt
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("Concept Test")
        .trim_matches(|value: char| value == '#' || value == '-' || value == '*')
        .trim();
    let sentence = candidate
        .split(['.', '\n'])
        .next()
        .unwrap_or(candidate)
        .trim();
    prompt_excerpt(sentence, 72)
}

fn prompt_excerpt(prompt: &str, max_chars: usize) -> String {
    let normalized = prompt.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= max_chars {
        return normalized;
    }
    let mut excerpt = normalized
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    excerpt.push_str("...");
    excerpt
}

fn stable_text_seed(scope: &str, text: &str, id: &str) -> u64 {
    let mut hasher = stable_hasher("concept_text_seed_v1");
    update_hash_str(&mut hasher, scope);
    update_hash_str(&mut hasher, text);
    update_hash_str(&mut hasher, id);
    finish_hash_u64(hasher)
}

fn slugify_non_empty(value: &str, fallback: &str) -> String {
    let mut slug = String::new();
    let mut last_was_separator = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
            last_was_separator = false;
        } else if !last_was_separator && !slug.is_empty() {
            slug.push('-');
            last_was_separator = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        fallback.into()
    } else {
        slug
    }
}

fn draft_goal(domain: ConceptDraftDomain) -> &'static str {
    match domain {
        ConceptDraftDomain::Gameplay => {
            "Find the concept angle that sustains play across early, repeated, and social sessions."
        }
        ConceptDraftDomain::DefenseProcurement => {
            "Find the concept angle that survives operator utility, technical review, and procurement pressure."
        }
        ConceptDraftDomain::EnterpriseSaas => {
            "Find the concept angle that survives proof, onboarding, security, and budget scrutiny."
        }
        ConceptDraftDomain::CivicPolicy => {
            "Find the concept angle that earns trust across supporters, persuadables, and skeptics."
        }
        ConceptDraftDomain::Creative => {
            "Find the concept angle that creates attention, emotional resonance, and repeat discussion."
        }
        ConceptDraftDomain::ConsumerProduct | ConceptDraftDomain::Business => {
            "Find the strongest market wedge before spending real launch budget."
        }
    }
}

fn draft_decision(domain: ConceptDraftDomain) -> &'static str {
    match domain {
        ConceptDraftDomain::Gameplay => "Pick the loop or design angle to prototype first.",
        ConceptDraftDomain::DefenseProcurement => {
            "Pick the product narrative to test with operators, technical evaluators, and buyers."
        }
        ConceptDraftDomain::EnterpriseSaas => "Pick the sales or onboarding control to test first.",
        ConceptDraftDomain::CivicPolicy => "Pick the message frame to test with a small panel.",
        ConceptDraftDomain::Creative => "Pick the story angle to test with a preview audience.",
        ConceptDraftDomain::ConsumerProduct | ConceptDraftDomain::Business => {
            "Pick a control, challenger, and next real-world learning test."
        }
    }
}

fn draft_time_steps(domain: ConceptDraftDomain) -> usize {
    match domain {
        ConceptDraftDomain::Gameplay | ConceptDraftDomain::DefenseProcurement => 10,
        _ => default_time_steps(),
    }
}

fn draft_success_metrics(domain: ConceptDraftDomain) -> Vec<String> {
    match domain {
        ConceptDraftDomain::Gameplay => vec![
            "activation_rate".into(),
            "retention_rate".into(),
            "referral_rate".into(),
        ],
        ConceptDraftDomain::DefenseProcurement | ConceptDraftDomain::EnterpriseSaas => vec![
            "signup_rate".into(),
            "activation_rate".into(),
            "retention_rate".into(),
        ],
        ConceptDraftDomain::CivicPolicy | ConceptDraftDomain::Creative => {
            vec![
                "click_rate".into(),
                "retention_rate".into(),
                "referral_rate".into(),
            ]
        }
        ConceptDraftDomain::ConsumerProduct | ConceptDraftDomain::Business => vec![
            "click_rate".into(),
            "signup_rate".into(),
            "activation_rate".into(),
        ],
    }
}

fn draft_channels(domain: ConceptDraftDomain, prompt: &str) -> Vec<String> {
    let text = prompt.to_ascii_lowercase();
    let mut channels = match domain {
        ConceptDraftDomain::Gameplay => vec!["gameplay", "in_game", "community"],
        ConceptDraftDomain::DefenseProcurement => {
            vec![
                "technical_brief",
                "demo",
                "field_trial",
                "procurement_review",
            ]
        }
        ConceptDraftDomain::EnterpriseSaas => {
            vec!["landing_page", "sales_call", "demo", "security_review"]
        }
        ConceptDraftDomain::CivicPolicy => vec!["short_form_video", "town_hall", "newsletter"],
        ConceptDraftDomain::Creative => vec!["trailer", "social_clip", "community"],
        ConceptDraftDomain::ConsumerProduct => vec!["landing_page", "tiktok", "reddit", "email"],
        ConceptDraftDomain::Business => vec!["landing_page", "linkedin", "reddit", "email"],
    }
    .into_iter()
    .map(str::to_string)
    .collect::<Vec<_>>();

    for (needle, channel) in [
        ("twitter", "twitter"),
        ("x.com", "twitter"),
        ("linkedin", "linkedin"),
        ("reddit", "reddit"),
        ("tiktok", "tiktok"),
        ("youtube", "youtube"),
        ("email", "email"),
        ("newsletter", "newsletter"),
        ("discord", "community"),
        ("sales call", "sales_call"),
        ("demo", "demo"),
    ] {
        if text.contains(needle) && !channels.iter().any(|value| value == channel) {
            channels.push(channel.into());
        }
    }

    channels
}

fn draft_baseline_touchpoints(
    domain: ConceptDraftDomain,
    channels: &[String],
) -> Vec<ConceptTouchpoint> {
    match domain {
        ConceptDraftDomain::Gameplay => vec![
            draft_touchpoint(
                "first_session",
                "First Session Hook",
                "gameplay",
                ConceptTouchpointFocus::Awareness,
                1.10,
                &[("novelty_seeking", 1.0)],
            ),
            draft_touchpoint(
                "repeat_loop",
                "Repeat Loop",
                "in_game",
                ConceptTouchpointFocus::Retention,
                1.05,
                &[("mastery_drive", 1.0)],
            ),
            draft_touchpoint(
                "social_share",
                "Social Share",
                "community",
                ConceptTouchpointFocus::Referral,
                0.95,
                &[("social_sharing", 1.0)],
            ),
        ],
        ConceptDraftDomain::DefenseProcurement => vec![
            draft_touchpoint(
                "mission_brief",
                "Mission Brief",
                "technical_brief",
                ConceptTouchpointFocus::Proof,
                1.10,
                &[("mission_criticality", 1.0)],
            ),
            draft_touchpoint(
                "field_demo",
                "Field Demo",
                "demo",
                ConceptTouchpointFocus::Conversion,
                1.05,
                &[("usability_pressure", 1.0)],
            ),
            draft_touchpoint(
                "procurement_review",
                "Procurement Review",
                "procurement_review",
                ConceptTouchpointFocus::ObjectionHandling,
                1.15,
                &[("budget_scrutiny", 1.0), ("risk_sensitivity", 0.8)],
            ),
        ],
        _ => {
            let first = channels
                .first()
                .cloned()
                .unwrap_or_else(|| "landing_page".into());
            let second = channels.get(1).cloned().unwrap_or_else(|| first.clone());
            vec![
                draft_touchpoint(
                    "hook",
                    "Hook",
                    &first,
                    ConceptTouchpointFocus::Awareness,
                    1.05,
                    &[("urgency", 0.8)],
                ),
                draft_touchpoint(
                    "proof",
                    "Proof Moment",
                    &second,
                    ConceptTouchpointFocus::Proof,
                    1.10,
                    &[("proof_hunger", 1.0)],
                ),
                draft_touchpoint(
                    "conversion",
                    "Conversion Ask",
                    &first,
                    ConceptTouchpointFocus::Conversion,
                    0.95,
                    &[("convenience_need", 0.8)],
                ),
            ]
        }
    }
}

fn draft_touchpoint(
    id: &str,
    label: &str,
    channel: &str,
    focus: ConceptTouchpointFocus,
    intensity: f64,
    trait_weights: &[(&str, f64)],
) -> ConceptTouchpoint {
    ConceptTouchpoint {
        id: Some(id.into()),
        label: label.into(),
        channel: Some(channel.into()),
        focus,
        intensity,
        target_segments: Vec::new(),
        variants: Vec::new(),
        trait_weights: trait_map(trait_weights),
    }
}

fn draft_segments(domain: ConceptDraftDomain) -> Vec<ConceptSegment> {
    match domain {
        ConceptDraftDomain::Gameplay => vec![
            draft_segment(
                "new_players",
                "New Players",
                0.38,
                &[
                    ("novelty_seeking", 0.86),
                    ("mastery_drive", 0.48),
                    ("social_sharing", 0.36),
                    ("frustration_sensitivity", 0.58),
                ],
                &["gameplay", "tiktok", "youtube"],
                &["unclear payoff", "early frustration"],
            ),
            draft_segment(
                "core_players",
                "Core Loop Players",
                0.37,
                &[
                    ("novelty_seeking", 0.52),
                    ("mastery_drive", 0.88),
                    ("social_sharing", 0.44),
                    ("fatigue_sensitivity", 0.56),
                ],
                &["gameplay", "in_game", "community"],
                &["repetitive loop", "shallow mastery"],
            ),
            draft_segment(
                "social_players",
                "Social Players",
                0.25,
                &[
                    ("novelty_seeking", 0.62),
                    ("mastery_drive", 0.62),
                    ("social_sharing", 0.91),
                    ("fatigue_sensitivity", 0.42),
                ],
                &["community", "in_game", "discord"],
                &["no friends playing", "low status payoff"],
            ),
        ],
        ConceptDraftDomain::DefenseProcurement => vec![
            draft_segment(
                "field_operators",
                "Field Operators",
                0.42,
                &[
                    ("mission_criticality", 0.92),
                    ("usability_pressure", 0.86),
                    ("proof_hunger", 0.74),
                    ("risk_sensitivity", 0.66),
                ],
                &["demo", "field_trial", "technical_brief"],
                &["cognitive load", "fragile in the field"],
            ),
            draft_segment(
                "technical_evaluators",
                "Technical Evaluators",
                0.32,
                &[
                    ("interoperability", 0.90),
                    ("proof_hunger", 0.93),
                    ("risk_sensitivity", 0.82),
                    ("budget_scrutiny", 0.64),
                ],
                &["technical_brief", "demo", "security_review"],
                &["integration risk", "unproven reliability"],
            ),
            draft_segment(
                "procurement_buyers",
                "Procurement Buyers",
                0.26,
                &[
                    ("budget_scrutiny", 0.92),
                    ("risk_sensitivity", 0.88),
                    ("proof_hunger", 0.82),
                    ("mission_criticality", 0.76),
                ],
                &["procurement_review", "sales_call", "technical_brief"],
                &["budget risk", "vendor risk"],
            ),
        ],
        ConceptDraftDomain::EnterpriseSaas => vec![
            draft_segment(
                "operators",
                "Operators",
                0.40,
                &[
                    ("proof_hunger", 0.82),
                    ("convenience_need", 0.80),
                    ("budget_scrutiny", 0.58),
                    ("risk_sensitivity", 0.54),
                ],
                &["landing_page", "linkedin", "demo"],
                &["workflow disruption", "setup time"],
            ),
            draft_segment(
                "technical_admins",
                "Technical Admins",
                0.32,
                &[
                    ("proof_hunger", 0.88),
                    ("risk_sensitivity", 0.84),
                    ("interoperability", 0.86),
                    ("privacy_sensitivity", 0.72),
                ],
                &["demo", "security_review", "technical_brief"],
                &["security risk", "integration work"],
            ),
            draft_segment(
                "budget_owners",
                "Budget Owners",
                0.28,
                &[
                    ("budget_scrutiny", 0.90),
                    ("proof_hunger", 0.78),
                    ("risk_sensitivity", 0.78),
                    ("urgency", 0.56),
                ],
                &["sales_call", "linkedin", "email"],
                &["unclear ROI", "budget timing"],
            ),
        ],
        _ => vec![
            draft_segment(
                "early_adopters",
                "Early Adopters",
                0.38,
                &[
                    ("proof_hunger", 0.70),
                    ("convenience_need", 0.78),
                    ("social_sharing", 0.56),
                    ("urgency", 0.82),
                ],
                &["landing_page", "reddit", "linkedin"],
                &["too early", "unclear value"],
            ),
            draft_segment(
                "pragmatic_buyers",
                "Pragmatic Buyers",
                0.37,
                &[
                    ("proof_hunger", 0.88),
                    ("convenience_need", 0.62),
                    ("budget_scrutiny", 0.70),
                    ("risk_sensitivity", 0.66),
                ],
                &["landing_page", "email", "linkedin"],
                &["not enough proof", "unclear ROI"],
            ),
            draft_segment(
                "skeptical_holdouts",
                "Skeptical Holdouts",
                0.25,
                &[
                    ("proof_hunger", 0.92),
                    ("privacy_sensitivity", 0.76),
                    ("risk_sensitivity", 0.88),
                    ("convenience_need", 0.46),
                ],
                &["reddit", "email", "newsletter"],
                &["trust", "privacy", "switching cost"],
            ),
        ],
    }
}

fn draft_segment(
    id: &str,
    name: &str,
    share_weight: f64,
    traits: &[(&str, f64)],
    channels: &[&str],
    objections: &[&str],
) -> ConceptSegment {
    ConceptSegment {
        id: id.into(),
        name: name.into(),
        share_weight,
        traits: trait_map(traits),
        channels: string_vec(channels),
        objections: string_vec(objections),
        target_count: None,
    }
}

fn draft_variants(
    domain: ConceptDraftDomain,
    prompt: &str,
    segments: &[ConceptSegment],
    channels: &[String],
) -> Vec<ConceptVariant> {
    let extracted = extract_angle_lines(prompt);
    let mut variants = extracted
        .iter()
        .take(5)
        .enumerate()
        .map(|(index, angle)| draft_variant_from_angle(domain, angle, index, segments, channels))
        .collect::<Vec<_>>();

    let mut defaults = default_draft_variants(domain, segments, channels);
    while variants.len() < 3 {
        let Some(next) = defaults.first().cloned() else {
            break;
        };
        defaults.remove(0);
        if variants
            .iter()
            .all(|variant| variant.id.as_str() != next.id.as_str())
        {
            variants.push(next);
        }
    }

    dedupe_variant_ids(&mut variants);
    variants
}

fn extract_angle_lines(prompt: &str) -> Vec<String> {
    let inline = extract_inline_angle_markers(prompt);
    if !inline.is_empty() {
        return inline;
    }

    dedupe_strings(
        prompt
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim().trim_start_matches(['-', '*']).trim();
                let (prefix, value) = trimmed.split_once(':')?;
                let prefix = prefix.trim().to_ascii_lowercase();
                let looks_like_angle = contains_any(
                    &prefix,
                    &["variant", "angle", "concept", "option", "design", "idea"],
                ) || prefix.len() <= 3;
                if looks_like_angle && !value.trim().is_empty() {
                    Some(prompt_excerpt(value.trim(), 110))
                } else {
                    None
                }
            })
            .collect(),
    )
}

fn extract_inline_angle_markers(prompt: &str) -> Vec<String> {
    let mut angles = Vec::new();
    for marker in [
        "Angle ", "angle ", "Variant ", "variant ", "Option ", "option ",
    ] {
        for part in prompt.split(marker).skip(1) {
            let Some((label, value)) = part.split_once(':') else {
                continue;
            };
            if label.trim().len() > 3 {
                continue;
            }
            let value = value.split(['\n', '.']).next().unwrap_or(value).trim();
            if !value.is_empty() {
                angles.push(prompt_excerpt(value, 110));
            }
        }
    }
    dedupe_strings(angles)
}

fn dedupe_strings(values: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    values
        .into_iter()
        .filter(|value| seen.insert(value.to_ascii_lowercase()))
        .collect()
}

fn draft_variant_from_angle(
    domain: ConceptDraftDomain,
    angle: &str,
    index: usize,
    segments: &[ConceptSegment],
    channels: &[String],
) -> ConceptVariant {
    let id = slugify_non_empty(angle, &format!("angle-{}", index + 1));
    ConceptVariant {
        id,
        name: title_case_excerpt(angle, 42),
        summary: Some(angle.into()),
        target_segments: segment_targets_for_text(segments, angle, index),
        channels: variant_channels_for_text(domain, angle, channels),
        trait_weights: trait_weights_for_text(domain, angle),
        strengths: strengths_for_text(domain, angle),
        risks: risks_for_text(domain, angle),
    }
}

fn default_draft_variants(
    domain: ConceptDraftDomain,
    segments: &[ConceptSegment],
    channels: &[String],
) -> Vec<ConceptVariant> {
    match domain {
        ConceptDraftDomain::Gameplay => vec![
            draft_variant(
                "novelty_loop",
                "Novelty Loop",
                "Lead with surprise, discovery, and first-session curiosity.",
                fallback_segment_targets(segments, 0),
                channels,
                &[("novelty_seeking", 1.0)],
                &["fresh mechanic", "fast first-session payoff"],
                &["may fatigue quickly"],
            ),
            draft_variant(
                "mastery_loop",
                "Mastery Loop",
                "Lead with skill growth, progression, and repeated-session depth.",
                fallback_segment_targets(segments, 1),
                channels,
                &[("mastery_drive", 1.0)],
                &["clear progression", "repeatable mastery"],
                &["may intimidate casual players"],
            ),
            draft_variant(
                "social_loop",
                "Social Loop",
                "Lead with status, community sharing, and group play.",
                fallback_segment_targets(segments, 2),
                channels,
                &[("social_sharing", 1.0)],
                &["community spread", "peer proof"],
                &["depends on network effects"],
            ),
        ],
        ConceptDraftDomain::DefenseProcurement => vec![
            draft_variant(
                "mission_outcome",
                "Mission Outcome Proof",
                "Lead with measurable mission impact and field reliability.",
                fallback_segment_targets(segments, 0),
                channels,
                &[("mission_criticality", 1.0), ("proof_hunger", 0.8)],
                &["mission proof", "field reliability"],
                &["requires credible evidence"],
            ),
            draft_variant(
                "integration_first",
                "Integration First",
                "Lead with interoperability, deployment path, and technical fit.",
                fallback_segment_targets(segments, 1),
                channels,
                &[("interoperability", 1.0), ("risk_sensitivity", 0.6)],
                &["integration path", "technical fit"],
                &["can feel less urgent"],
            ),
            draft_variant(
                "procurement_risk_reduction",
                "Procurement Risk Reduction",
                "Lead with budget control, vendor risk reduction, and review readiness.",
                fallback_segment_targets(segments, 2),
                channels,
                &[("budget_scrutiny", 1.0), ("risk_sensitivity", 0.9)],
                &["budget risk", "vendor risk", "review ready"],
                &["less exciting to operators"],
            ),
        ],
        ConceptDraftDomain::EnterpriseSaas => vec![
            draft_variant(
                "operator_roi",
                "Operator ROI",
                "Lead with workflow value and measurable operator outcomes.",
                fallback_segment_targets(segments, 0),
                channels,
                &[("proof_hunger", 0.9), ("convenience_need", 0.8)],
                &["workflow proof", "clear ROI"],
                &["needs concrete benchmark"],
            ),
            draft_variant(
                "admin_trust",
                "Admin Trust",
                "Lead with security, integrations, and low-risk adoption.",
                fallback_segment_targets(segments, 1),
                channels,
                &[("risk_sensitivity", 1.0), ("interoperability", 0.9)],
                &["security review", "integration path"],
                &["can slow conversion"],
            ),
            draft_variant(
                "budget_case",
                "Budget Case",
                "Lead with payback, budget fit, and timing urgency.",
                fallback_segment_targets(segments, 2),
                channels,
                &[("budget_scrutiny", 1.0), ("urgency", 0.6)],
                &["budget fit", "payback"],
                &["may underplay product delight"],
            ),
        ],
        _ => vec![
            draft_variant(
                "proof_led",
                "Proof-Led Outcome Case",
                "Lead with evidence, benchmarks, and concrete outcomes.",
                fallback_segment_targets(segments, 1),
                channels,
                &[("proof_hunger", 1.0)],
                &["proof", "case study", "evidence"],
                &["slower to explain"],
            ),
            draft_variant(
                "frictionless_value",
                "Frictionless First Value",
                "Lead with speed, ease, and a quick first win.",
                fallback_segment_targets(segments, 0),
                channels,
                &[("convenience_need", 1.0), ("urgency", 0.6)],
                &["fast setup", "quick win"],
                &["may feel shallow"],
            ),
            draft_variant(
                "trust_first",
                "Trust and Risk Reversal",
                "Lead with safety, privacy, and objection handling.",
                fallback_segment_targets(segments, 2),
                channels,
                &[("risk_sensitivity", 1.0), ("privacy_sensitivity", 0.7)],
                &["trust", "privacy", "risk reversal"],
                &["can reduce urgency"],
            ),
        ],
    }
}

fn draft_variant(
    id: &str,
    name: &str,
    summary: &str,
    target_segments: Vec<String>,
    channels: &[String],
    trait_weights: &[(&str, f64)],
    strengths: &[&str],
    risks: &[&str],
) -> ConceptVariant {
    ConceptVariant {
        id: id.into(),
        name: name.into(),
        summary: Some(summary.into()),
        target_segments,
        channels: channels.to_vec(),
        trait_weights: trait_map(trait_weights),
        strengths: string_vec(strengths),
        risks: string_vec(risks),
    }
}

fn dedupe_variant_ids(variants: &mut [ConceptVariant]) {
    let mut seen = BTreeMap::<String, usize>::new();
    for variant in variants {
        let count = seen.entry(variant.id.clone()).or_default();
        if *count > 0 {
            variant.id = format!("{}-{}", variant.id, *count + 1);
        }
        *count += 1;
    }
}

fn title_case_excerpt(value: &str, max_chars: usize) -> String {
    prompt_excerpt(value, max_chars)
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => {
                    let mut output = first.to_ascii_uppercase().to_string();
                    output.push_str(chars.as_str());
                    output
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn variant_channels_for_text(
    domain: ConceptDraftDomain,
    text: &str,
    channels: &[String],
) -> Vec<String> {
    let lowered = text.to_ascii_lowercase();
    let mut selected = channels
        .iter()
        .filter(|channel| lowered.contains(&channel.replace('_', " ")))
        .cloned()
        .collect::<Vec<_>>();
    if selected.is_empty() {
        selected = match domain {
            ConceptDraftDomain::Gameplay => ["gameplay", "in_game", "community"]
                .into_iter()
                .filter(|channel| channels.iter().any(|value| value.as_str() == *channel))
                .map(str::to_string)
                .collect(),
            ConceptDraftDomain::DefenseProcurement => {
                ["technical_brief", "demo", "procurement_review"]
                    .into_iter()
                    .filter(|channel| channels.iter().any(|value| value.as_str() == *channel))
                    .map(str::to_string)
                    .collect()
            }
            _ => channels.iter().take(3).cloned().collect(),
        };
    }
    if selected.is_empty() {
        channels.to_vec()
    } else {
        selected
    }
}

fn segment_targets_for_text(
    segments: &[ConceptSegment],
    text: &str,
    fallback_index: usize,
) -> Vec<String> {
    let lowered = text.to_ascii_lowercase();
    let matches = segments
        .iter()
        .filter(|segment| {
            lowered.contains(&segment.id.replace('_', " "))
                || lowered.contains(&segment.name.to_ascii_lowercase())
                || segment
                    .traits
                    .keys()
                    .any(|trait_name| lowered.contains(&trait_name.replace('_', " ")))
        })
        .map(|segment| segment.id.clone())
        .collect::<Vec<_>>();
    if matches.is_empty() {
        fallback_segment_targets(segments, fallback_index)
    } else {
        matches
    }
}

fn fallback_segment_targets(segments: &[ConceptSegment], index: usize) -> Vec<String> {
    if segments.is_empty() {
        Vec::new()
    } else {
        vec![segments[index % segments.len()].id.clone()]
    }
}

fn trait_weights_for_text(domain: ConceptDraftDomain, text: &str) -> BTreeMap<String, f64> {
    let lowered = text.to_ascii_lowercase();
    let mut weights = BTreeMap::new();
    for (needles, trait_name, weight) in [
        (
            &["proof", "evidence", "case", "data", "benchmark"][..],
            "proof_hunger",
            1.0,
        ),
        (
            &["privacy", "secure", "safe", "risk", "trust"][..],
            "risk_sensitivity",
            0.9,
        ),
        (
            &["fast", "easy", "simple", "friction", "setup"][..],
            "convenience_need",
            1.0,
        ),
        (
            &["viral", "community", "share", "social", "referral"][..],
            "social_sharing",
            1.0,
        ),
        (
            &["budget", "price", "cost", "procurement", "roi"][..],
            "budget_scrutiny",
            1.0,
        ),
        (
            &["mission", "field", "operator", "reliable"][..],
            "mission_criticality",
            1.0,
        ),
        (
            &["integration", "interoperability", "api", "deploy"][..],
            "interoperability",
            1.0,
        ),
        (
            &["novel", "surprise", "fresh", "discovery"][..],
            "novelty_seeking",
            1.0,
        ),
        (
            &["mastery", "skill", "progression", "depth"][..],
            "mastery_drive",
            1.0,
        ),
    ] {
        if contains_any(&lowered, needles) {
            weights.insert(trait_name.into(), weight);
        }
    }
    if weights.is_empty() {
        let fallback = match domain {
            ConceptDraftDomain::Gameplay => "novelty_seeking",
            ConceptDraftDomain::DefenseProcurement => "mission_criticality",
            ConceptDraftDomain::EnterpriseSaas => "proof_hunger",
            ConceptDraftDomain::CivicPolicy | ConceptDraftDomain::Creative => "social_sharing",
            ConceptDraftDomain::ConsumerProduct | ConceptDraftDomain::Business => "proof_hunger",
        };
        weights.insert(fallback.into(), 1.0);
    }
    weights
}

fn strengths_for_text(domain: ConceptDraftDomain, text: &str) -> Vec<String> {
    let lowered = text.to_ascii_lowercase();
    let mut strengths = Vec::new();
    for (needle, strength) in [
        ("proof", "proof"),
        ("evidence", "evidence"),
        ("privacy", "privacy"),
        ("secure", "security"),
        ("fast", "speed"),
        ("easy", "ease"),
        ("community", "community"),
        ("mission", "mission impact"),
        ("integration", "integration path"),
        ("mastery", "mastery"),
        ("novel", "novelty"),
    ] {
        if lowered.contains(needle) {
            strengths.push(strength.into());
        }
    }
    if strengths.is_empty() {
        strengths.push(
            match domain {
                ConceptDraftDomain::Gameplay => "clear play loop",
                ConceptDraftDomain::DefenseProcurement => "mission relevance",
                ConceptDraftDomain::EnterpriseSaas => "operator value",
                ConceptDraftDomain::CivicPolicy => "message clarity",
                ConceptDraftDomain::Creative => "audience resonance",
                ConceptDraftDomain::ConsumerProduct | ConceptDraftDomain::Business => {
                    "clear value proposition"
                }
            }
            .into(),
        );
    }
    strengths
}

fn risks_for_text(domain: ConceptDraftDomain, text: &str) -> Vec<String> {
    let lowered = text.to_ascii_lowercase();
    let mut risks = Vec::new();
    if contains_any(&lowered, &["cheap", "discount", "low price"]) {
        risks.push("may attract low-intent buyers".into());
    }
    if contains_any(&lowered, &["complex", "technical", "advanced"]) {
        risks.push("may increase onboarding friction".into());
    }
    if risks.is_empty() {
        risks.push(
            match domain {
                ConceptDraftDomain::Gameplay => "may not sustain repeated sessions",
                ConceptDraftDomain::DefenseProcurement => "may require stronger field evidence",
                ConceptDraftDomain::EnterpriseSaas => "may need sharper ROI proof",
                ConceptDraftDomain::CivicPolicy => "may polarize skeptical audiences",
                ConceptDraftDomain::Creative => "may not convert attention into advocacy",
                ConceptDraftDomain::ConsumerProduct | ConceptDraftDomain::Business => {
                    "needs real-world validation"
                }
            }
            .into(),
        );
    }
    risks
}

fn draft_matrix_cases(
    domain: ConceptDraftDomain,
    prompt: &str,
    case_count: Option<usize>,
    allowed_presets: &[ConceptScenarioPreset],
    seed_base: u64,
    population: &ConceptPopulationConfig,
) -> Vec<ConceptTestMatrixCase> {
    let presets = selected_presets(domain, prompt, allowed_presets);
    let limit = case_count
        .unwrap_or_else(|| presets.len().min(6))
        .min(presets.len());
    presets
        .into_iter()
        .take(limit)
        .enumerate()
        .map(|(index, preset)| ConceptTestMatrixCase {
            id: preset_slug(preset).into(),
            name: preset_name(preset).into(),
            description: Some(preset_description(preset).into()),
            preset: Some(preset),
            scenario: None,
            seed_base: Some(seed_base.wrapping_add(101 + (index as u64 * 101))),
            population: Some(population.clone()),
            segments: Vec::new(),
            observed_outcomes: Vec::new(),
        })
        .collect()
}

fn selected_presets(
    domain: ConceptDraftDomain,
    prompt: &str,
    allowed_presets: &[ConceptScenarioPreset],
) -> Vec<ConceptScenarioPreset> {
    if !allowed_presets.is_empty() {
        return allowed_presets.to_vec();
    }

    let mut presets = Vec::new();
    let lowered = prompt.to_ascii_lowercase();
    if contains_any(
        &lowered,
        &["acquisition", "waitlist", "paid ad", "growth", "cheap"],
    ) {
        push_unique_preset(&mut presets, ConceptScenarioPreset::CheapAcquisition);
    }
    if contains_any(&lowered, &["trust", "privacy", "security", "credibility"]) {
        push_unique_preset(&mut presets, ConceptScenarioPreset::TrustCollapse);
    }
    if contains_any(&lowered, &["retention", "habit", "repeat", "long term"]) {
        push_unique_preset(&mut presets, ConceptScenarioPreset::RetentionLoop);
    }
    if contains_any(&lowered, &["referral", "community", "viral", "share"]) {
        push_unique_preset(&mut presets, ConceptScenarioPreset::ReferralLoop);
    }
    if contains_any(&lowered, &["price", "pricing", "budget", "cost", "roi"]) {
        push_unique_preset(&mut presets, ConceptScenarioPreset::PricingPressure);
    }
    if contains_any(&lowered, &["onboarding", "setup", "friction", "adoption"]) {
        push_unique_preset(&mut presets, ConceptScenarioPreset::OnboardingFriction);
    }
    if contains_any(
        &lowered,
        &[
            "procurement",
            "military",
            "defense",
            "dod",
            "operator",
            "mission",
        ],
    ) {
        push_unique_preset(&mut presets, ConceptScenarioPreset::ProcurementSkepticism);
    }
    if contains_any(
        &lowered,
        &["gameplay", "player", "session", "fatigue", "game"],
    ) {
        push_unique_preset(&mut presets, ConceptScenarioPreset::GameplayFatigue);
    }

    for preset in default_presets_for_domain(domain) {
        push_unique_preset(&mut presets, preset);
    }
    presets
}

fn default_presets_for_domain(domain: ConceptDraftDomain) -> Vec<ConceptScenarioPreset> {
    match domain {
        ConceptDraftDomain::Gameplay => vec![
            ConceptScenarioPreset::GameplayFatigue,
            ConceptScenarioPreset::RetentionLoop,
            ConceptScenarioPreset::ReferralLoop,
            ConceptScenarioPreset::OnboardingFriction,
            ConceptScenarioPreset::TrustCollapse,
            ConceptScenarioPreset::PricingPressure,
        ],
        ConceptDraftDomain::DefenseProcurement => vec![
            ConceptScenarioPreset::ProcurementSkepticism,
            ConceptScenarioPreset::PricingPressure,
            ConceptScenarioPreset::TrustCollapse,
            ConceptScenarioPreset::OnboardingFriction,
            ConceptScenarioPreset::RetentionLoop,
            ConceptScenarioPreset::CheapAcquisition,
        ],
        ConceptDraftDomain::EnterpriseSaas => vec![
            ConceptScenarioPreset::ProcurementSkepticism,
            ConceptScenarioPreset::PricingPressure,
            ConceptScenarioPreset::OnboardingFriction,
            ConceptScenarioPreset::TrustCollapse,
            ConceptScenarioPreset::RetentionLoop,
            ConceptScenarioPreset::CheapAcquisition,
        ],
        _ => vec![
            ConceptScenarioPreset::CheapAcquisition,
            ConceptScenarioPreset::OnboardingFriction,
            ConceptScenarioPreset::PricingPressure,
            ConceptScenarioPreset::TrustCollapse,
            ConceptScenarioPreset::RetentionLoop,
            ConceptScenarioPreset::ReferralLoop,
        ],
    }
}

fn push_unique_preset(presets: &mut Vec<ConceptScenarioPreset>, preset: ConceptScenarioPreset) {
    if !presets.contains(&preset) {
        presets.push(preset);
    }
}

fn preset_slug(preset: ConceptScenarioPreset) -> &'static str {
    match preset {
        ConceptScenarioPreset::CheapAcquisition => "cheap_acquisition",
        ConceptScenarioPreset::TrustCollapse => "trust_collapse",
        ConceptScenarioPreset::RetentionLoop => "retention_loop",
        ConceptScenarioPreset::ReferralLoop => "referral_loop",
        ConceptScenarioPreset::PricingPressure => "pricing_pressure",
        ConceptScenarioPreset::OnboardingFriction => "onboarding_friction",
        ConceptScenarioPreset::ProcurementSkepticism => "procurement_skepticism",
        ConceptScenarioPreset::GameplayFatigue => "gameplay_fatigue",
    }
}

fn preset_name(preset: ConceptScenarioPreset) -> &'static str {
    match preset {
        ConceptScenarioPreset::CheapAcquisition => "Cheap Acquisition",
        ConceptScenarioPreset::TrustCollapse => "Trust Collapse",
        ConceptScenarioPreset::RetentionLoop => "Retention Loop",
        ConceptScenarioPreset::ReferralLoop => "Referral Loop",
        ConceptScenarioPreset::PricingPressure => "Pricing Pressure",
        ConceptScenarioPreset::OnboardingFriction => "Onboarding Friction",
        ConceptScenarioPreset::ProcurementSkepticism => "Procurement Skepticism",
        ConceptScenarioPreset::GameplayFatigue => "Gameplay Fatigue",
    }
}

fn preset_description(preset: ConceptScenarioPreset) -> &'static str {
    match preset {
        ConceptScenarioPreset::CheapAcquisition => {
            "Tests whether the concept can create low-cost initial demand."
        }
        ConceptScenarioPreset::TrustCollapse => {
            "Tests how the concept survives a credibility, privacy, or trust shock."
        }
        ConceptScenarioPreset::RetentionLoop => {
            "Tests whether the concept can create repeated use after the first win."
        }
        ConceptScenarioPreset::ReferralLoop => {
            "Tests whether satisfied users would share or advocate for the concept."
        }
        ConceptScenarioPreset::PricingPressure => {
            "Tests how the concept holds up under price, budget, or ROI scrutiny."
        }
        ConceptScenarioPreset::OnboardingFriction => {
            "Tests whether setup or adoption friction breaks momentum."
        }
        ConceptScenarioPreset::ProcurementSkepticism => {
            "Tests technical, budget, risk, and formal buyer skepticism."
        }
        ConceptScenarioPreset::GameplayFatigue => {
            "Tests whether a game or loop remains compelling after repeated sessions."
        }
    }
}

fn trait_map(values: &[(&str, f64)]) -> BTreeMap<String, f64> {
    values
        .iter()
        .map(|(name, value)| ((*name).into(), *value))
        .collect()
}

fn string_vec(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).into()).collect()
}

fn build_comparison_entry(
    input: &ConceptTestComparisonInput,
) -> Result<ConceptTestComparisonEntry, ConceptTestCompareError> {
    let top = input
        .result
        .ranked_variants
        .first()
        .ok_or_else(|| ConceptTestCompareError::EmptyResult(input.source.clone()))?;
    let runner_up = input.result.ranked_variants.get(1);
    Ok(ConceptTestComparisonEntry {
        source: input.source.clone(),
        concept_test_id: input.result.concept_test_id.clone(),
        name: input.result.name.clone(),
        scenario_name: input.result.scenario.name.clone(),
        total_population: input.result.total_population,
        recommended_variant_id: input.result.recommended_variant_id.clone(),
        top_variant_id: top.variant_id.clone(),
        top_variant_score: top.overall_score,
        runner_up_variant_id: runner_up.map(|variant| variant.variant_id.clone()),
        runner_up_variant_score: runner_up.map(|variant| variant.overall_score),
        score_gap_vs_runner_up: runner_up
            .map(|variant| top.overall_score as i32 - variant.overall_score as i32),
        top_variant_funnel: top.funnel.clone(),
        metric_deltas: Vec::new(),
        segment_winners: input
            .result
            .segment_summaries
            .iter()
            .map(|segment| ConceptSegmentWinner {
                segment_id: segment.segment_id.clone(),
                segment_name: segment.segment_name.clone(),
                best_variant_id: segment.best_variant_id.clone(),
                best_score: segment.best_score,
            })
            .collect(),
        calibration_status: calibration_status(&input.result),
    })
}

fn fill_metric_deltas(entries: &mut [ConceptTestComparisonEntry]) {
    let metric_values = entries
        .iter()
        .map(|entry| {
            (
                entry.name.clone(),
                vec![
                    ("overall_score".to_string(), entry.top_variant_score as f64),
                    (
                        "click_rate".to_string(),
                        entry.top_variant_funnel.click_rate,
                    ),
                    (
                        "signup_rate".to_string(),
                        entry.top_variant_funnel.signup_rate,
                    ),
                    (
                        "activation_rate".to_string(),
                        entry.top_variant_funnel.activation_rate,
                    ),
                    (
                        "retention_rate".to_string(),
                        entry.top_variant_funnel.retention_rate,
                    ),
                    (
                        "referral_rate".to_string(),
                        entry.top_variant_funnel.referral_rate,
                    ),
                ],
            )
        })
        .collect::<Vec<_>>();

    for entry in entries {
        let mut deltas = Vec::new();
        for (label, value) in [
            ("overall_score", entry.top_variant_score as f64),
            ("click_rate", entry.top_variant_funnel.click_rate),
            ("signup_rate", entry.top_variant_funnel.signup_rate),
            ("activation_rate", entry.top_variant_funnel.activation_rate),
            ("retention_rate", entry.top_variant_funnel.retention_rate),
            ("referral_rate", entry.top_variant_funnel.referral_rate),
        ] {
            let values = metric_values
                .iter()
                .filter_map(|(name, metrics)| {
                    metrics
                        .iter()
                        .find(|(metric_label, _)| metric_label == label)
                        .map(|(_, metric_value)| (name.clone(), *metric_value))
                })
                .collect::<Vec<_>>();
            let average = values.iter().map(|(_, value)| *value).sum::<f64>() / values.len() as f64;
            let leader = values
                .iter()
                .map(|(_, value)| *value)
                .fold(f64::NEG_INFINITY, f64::max);
            let rank = values.iter().filter(|(_, other)| *other > value).count() + 1;
            let leading_tests = values
                .iter()
                .filter(|(_, other)| (*other - leader).abs() <= f64::EPSILON)
                .map(|(name, _)| name.clone())
                .collect::<Vec<_>>();

            deltas.push(ConceptMetricDelta {
                label: label.into(),
                value,
                delta_vs_compare_average: value - average,
                delta_vs_compare_leader: value - leader,
                compare_set_rank: rank,
                compare_set_size: values.len(),
                leading_tests,
            });
        }
        deltas.sort_by(|left, right| {
            right
                .delta_vs_compare_average
                .total_cmp(&left.delta_vs_compare_average)
                .then_with(|| left.label.cmp(&right.label))
        });
        entry.metric_deltas = deltas;
    }
}

fn build_comparison_recommendation(entries: &[ConceptTestComparisonEntry]) -> Vec<String> {
    let mut recommendation = Vec::new();
    if let Some(control) = entries.first() {
        recommendation.push(format!(
            "Use `{}` as the control test; `{}` won with `{}` points.",
            control.name, control.top_variant_id, control.top_variant_score
        ));
    }
    if let Some(challenger) = entries.get(1) {
        recommendation.push(format!(
            "Use `{}` as the challenger; its top variant `{}` scored `{}`.",
            challenger.name, challenger.top_variant_id, challenger.top_variant_score
        ));
    }
    if let Some(best_gap) = entries
        .iter()
        .filter_map(|entry| entry.score_gap_vs_runner_up.map(|gap| (entry, gap)))
        .max_by_key(|(_, gap)| *gap)
    {
        recommendation.push(format!(
            "`{}` has the clearest internal winner with a `{}` point gap over its runner-up.",
            best_gap.0.name, best_gap.1
        ));
    }
    if entries
        .iter()
        .all(|entry| entry.calibration_status == ConceptCalibrationStatus::Uncalibrated)
    {
        recommendation.push(
            "All compared tests are uncalibrated, so treat this as a prioritization aid rather than a forecast."
                .into(),
        );
    }
    recommendation
}

fn build_repeated_winner_patterns(entries: &[ConceptTestComparisonEntry]) -> Vec<String> {
    let mut counts = BTreeMap::<String, usize>::new();
    for entry in entries {
        *counts.entry(entry.top_variant_id.clone()).or_default() += 1;
    }
    counts
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|(variant_id, count)| {
            format!("`{variant_id}` won in `{count}` concept tests, which suggests a repeated pattern worth preserving.")
        })
        .collect()
}

fn calibration_status(result: &ConceptTestResult) -> ConceptCalibrationStatus {
    if result.calibration_summary.is_empty() && result.touchpoint_calibration_summary.is_empty() {
        return ConceptCalibrationStatus::Uncalibrated;
    }
    if result
        .calibration_summary
        .iter()
        .chain(result.touchpoint_calibration_summary.iter())
        .any(|summary| summary.usable_observed_records > 0)
    {
        ConceptCalibrationStatus::CalibratedSignal
    } else {
        ConceptCalibrationStatus::ObservedWithoutRates
    }
}

fn validate_request(request: &ConceptTestRequest) -> Result<(), ConceptTestError> {
    validate_non_empty(&request.id, ConceptTestError::EmptyId)?;
    validate_non_empty(&request.name, ConceptTestError::EmptyName)?;
    validate_non_empty(&request.scenario.id, ConceptTestError::EmptyScenarioId)?;
    validate_non_empty(&request.scenario.name, ConceptTestError::EmptyScenarioName)?;
    if request.population.target_count == 0 {
        return Err(ConceptTestError::ZeroPopulation);
    }
    if request.scenario.time_steps == 0 {
        return Err(ConceptTestError::ZeroTimeSteps);
    }
    if request.segments.is_empty() {
        return Err(ConceptTestError::NoSegments);
    }
    if request.variants.is_empty() {
        return Err(ConceptTestError::NoVariants);
    }

    let mut segment_ids = BTreeSet::new();
    for segment in &request.segments {
        validate_non_empty(&segment.id, ConceptTestError::EmptySegmentId)?;
        validate_non_empty(&segment.name, ConceptTestError::EmptySegmentName)?;
        if !segment_ids.insert(segment.id.clone()) {
            return Err(ConceptTestError::DuplicateSegmentId(segment.id.clone()));
        }
        if !segment.share_weight.is_finite() || segment.share_weight <= 0.0 {
            return Err(ConceptTestError::InvalidSegmentShare {
                segment_id: segment.id.clone(),
                value: segment.share_weight,
            });
        }
        if segment.target_count == Some(0) {
            return Err(ConceptTestError::InvalidSegmentTargetCount(
                segment.id.clone(),
            ));
        }
        for (trait_name, value) in &segment.traits {
            validate_non_empty(
                trait_name,
                ConceptTestError::InvalidTraitName(segment.id.clone()),
            )?;
            if !value.is_finite() || !(0.0..=1.0).contains(value) {
                return Err(ConceptTestError::InvalidTraitValue {
                    segment_id: segment.id.clone(),
                    trait_name: trait_name.clone(),
                    value: *value,
                });
            }
        }
    }

    let mut variant_ids = BTreeSet::new();
    for variant in &request.variants {
        validate_non_empty(&variant.id, ConceptTestError::EmptyVariantId)?;
        validate_non_empty(&variant.name, ConceptTestError::EmptyVariantName)?;
        if !variant_ids.insert(variant.id.clone()) {
            return Err(ConceptTestError::DuplicateVariantId(variant.id.clone()));
        }
        for segment_id in &variant.target_segments {
            if !segment_ids.contains(segment_id) {
                return Err(ConceptTestError::UnknownTargetSegment {
                    variant_id: variant.id.clone(),
                    segment_id: segment_id.clone(),
                });
            }
        }
        for (trait_name, value) in &variant.trait_weights {
            validate_non_empty(
                trait_name,
                ConceptTestError::InvalidVariantTraitName(variant.id.clone()),
            )?;
            if !value.is_finite() {
                return Err(ConceptTestError::InvalidVariantTraitWeight {
                    variant_id: variant.id.clone(),
                    trait_name: trait_name.clone(),
                    value: *value,
                });
            }
        }
    }

    let mut touchpoint_ids = BTreeSet::new();
    for (index, touchpoint) in request.scenario.touchpoints.iter().enumerate() {
        if let Some(id) = &touchpoint.id {
            validate_non_empty(id, ConceptTestError::EmptyTouchpointId)?;
        }
        validate_non_empty(&touchpoint.label, ConceptTestError::EmptyTouchpointLabel)?;
        let touchpoint_id = touchpoint_key(index, touchpoint);
        if !touchpoint_ids.insert(touchpoint_id.clone()) {
            return Err(ConceptTestError::DuplicateTouchpointId(touchpoint_id));
        }
        if let Some(channel) = &touchpoint.channel {
            validate_non_empty(channel, ConceptTestError::EmptyTouchpointChannel)?;
        }
        if !touchpoint.intensity.is_finite() || touchpoint.intensity < 0.0 {
            return Err(ConceptTestError::InvalidTouchpointIntensity {
                label: touchpoint.label.clone(),
                value: touchpoint.intensity,
            });
        }
        for segment_id in &touchpoint.target_segments {
            if !segment_ids.contains(segment_id) {
                return Err(ConceptTestError::UnknownTouchpointSegment {
                    label: touchpoint.label.clone(),
                    segment_id: segment_id.clone(),
                });
            }
        }
        for variant_id in &touchpoint.variants {
            if !variant_ids.contains(variant_id) {
                return Err(ConceptTestError::UnknownTouchpointVariant {
                    label: touchpoint.label.clone(),
                    variant_id: variant_id.clone(),
                });
            }
        }
        for (trait_name, value) in &touchpoint.trait_weights {
            validate_non_empty(
                trait_name,
                ConceptTestError::InvalidTouchpointTraitName(touchpoint.label.clone()),
            )?;
            if !value.is_finite() {
                return Err(ConceptTestError::InvalidTouchpointTraitWeight {
                    label: touchpoint.label.clone(),
                    trait_name: trait_name.clone(),
                    value: *value,
                });
            }
        }
    }

    for outcome in &request.observed_outcomes {
        if !variant_ids.contains(&outcome.variant_id) {
            return Err(ConceptTestError::UnknownObservedVariant(
                outcome.variant_id.clone(),
            ));
        }
        if let Some(segment_id) = &outcome.segment_id {
            if !segment_ids.contains(segment_id) {
                return Err(ConceptTestError::UnknownObservedSegment(segment_id.clone()));
            }
        }
        if let Some(touchpoint_id) = &outcome.touchpoint_id {
            if !touchpoint_ids.contains(touchpoint_id) {
                return Err(ConceptTestError::UnknownObservedTouchpoint {
                    variant_id: outcome.variant_id.clone(),
                    touchpoint_id: touchpoint_id.clone(),
                });
            }
        }
        for (field, value) in [
            ("click_rate", outcome.click_rate),
            ("signup_rate", outcome.signup_rate),
            ("activation_rate", outcome.activation_rate),
            ("retention_rate", outcome.retention_rate),
            ("referral_rate", outcome.referral_rate),
        ] {
            if let Some(value) = value {
                if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                    return Err(ConceptTestError::InvalidObservedRate {
                        variant_id: outcome.variant_id.clone(),
                        field,
                        value,
                    });
                }
            }
        }
    }

    Ok(())
}

fn validate_non_empty(value: &str, error: ConceptTestError) -> Result<(), ConceptTestError> {
    if value.trim().is_empty() {
        Err(error)
    } else {
        Ok(())
    }
}

fn to_population_blueprint(segment: &ConceptSegment) -> PopulationSegmentBlueprint {
    let mut blueprint = PopulationSegmentBlueprint {
        id: segment.id.clone(),
        name: segment.name.clone(),
        default_stage: SegmentStage::CuriousObserver,
        target_count: segment
            .target_count
            // PopulationGenerator normalizes these hints as ratios; 10,000 keeps
            // fractional share weights precise without forcing the final run size.
            .unwrap_or_else(|| (segment.share_weight * 10_000.0).round().max(1.0) as usize),
        channel_preferences: segment
            .channels
            .iter()
            .map(|channel| ChannelPreference::new(Channel::Custom(channel.clone()), 1.0))
            .collect(),
        objections: segment
            .objections
            .iter()
            .map(|objection| Objection {
                objection_type: ObjectionType::Custom(objection.clone()),
                activation_probability: 0.55,
                severity: 0.58,
                resolvable_via_social_proof: true,
            })
            .collect(),
        budget: Budget {
            monthly_min: 0.0,
            monthly_max: 250.0,
            price_sensitive: true,
            prefers_annual: false,
        },
        ..PopulationSegmentBlueprint::default()
    };

    for (trait_name, value) in &segment.traits {
        let config = normalized_trait_config(*value);
        match trait_name.as_str() {
            "proof_hunger" => blueprint.proof_hunger = config,
            "privacy_sensitivity" => blueprint.privacy_sensitivity = config,
            "wearable_ownership" => blueprint.wearable_ownership = config,
            "peptide_openness" => blueprint.peptide_openness = config,
            "glp1_familiarity" => blueprint.glp1_familiarity = config,
            "logging_tolerance" => blueprint.logging_tolerance = config,
            "rigor_threshold" => blueprint.rigor_threshold = config,
            _ => {
                blueprint.extra_traits.insert(trait_name.clone(), config);
            }
        }
    }

    blueprint
}

fn normalized_trait_config(mean: f64) -> TraitDistributionConfig {
    TraitDistributionConfig {
        distribution_type: "truncated_normal".into(),
        params: BTreeMap::from([
            ("mean".into(), mean.clamp(0.0, 1.0)),
            ("stddev".into(), 0.14),
            ("lo".into(), 0.0),
            ("hi".into(), 1.0),
        ]),
    }
}

fn score_buyer_against_variant(
    buyer: &Buyer,
    segment: &ConceptSegment,
    variant: &ConceptVariant,
    scenario: &ConceptScenario,
    seed_base: u64,
) -> ScoredVariant {
    let components = base_components(buyer, segment, variant, scenario);
    let base_score = weighted_component_score(&components);

    if scenario.touchpoints.is_empty() {
        return ScoredVariant {
            variant_id: variant.id.clone(),
            score: base_score,
            trait_fit: components.trait_fit,
            channel_fit: components.channel_fit,
            trust: components.trust,
            objection_pressure: components.objection_pressure,
            funnel: build_funnel(
                base_score,
                components.trait_fit,
                components.channel_fit,
                components.trust,
                components.objection_pressure,
                trait_value(buyer, "social_sharing"),
                components.time_factor,
                stable_seed(seed_base, &buyer.id.0, &variant.id),
            ),
            touchpoints: Vec::new(),
        };
    }

    let mut touchpoints = Vec::new();
    let mut prior_score = base_score;
    let mut trust_memory = components.trust;
    let mut objection_memory = components.objection_pressure;

    for (index, touchpoint) in scenario.touchpoints.iter().enumerate() {
        let touchpoint_score = score_touchpoint(
            buyer,
            segment,
            variant,
            scenario,
            touchpoint,
            index,
            components,
            prior_score,
            trust_memory,
            objection_memory,
            seed_base,
        );
        prior_score = clamp01((prior_score * 0.58) + (touchpoint_score.score * 0.42));
        trust_memory = clamp01((trust_memory * 0.70) + (touchpoint_score.trust * 0.30));
        objection_memory =
            clamp01((objection_memory * 0.74) + (touchpoint_score.objection_pressure * 0.26));
        touchpoints.push(touchpoint_score);
    }

    let average_touchpoint_score =
        touchpoint_average(&touchpoints, |score| score.score).unwrap_or(base_score);
    let final_score =
        clamp01((base_score * 0.34) + (average_touchpoint_score * 0.44) + (prior_score * 0.22));
    let final_trait_fit =
        touchpoint_average(&touchpoints, |score| score.trait_fit).unwrap_or(components.trait_fit);
    let final_channel_fit = touchpoint_average(&touchpoints, |score| score.channel_fit)
        .unwrap_or(components.channel_fit);
    let final_trust =
        touchpoint_average(&touchpoints, |score| score.trust).unwrap_or(components.trust);
    let final_objection_pressure =
        touchpoint_average(&touchpoints, |score| score.objection_pressure)
            .unwrap_or(components.objection_pressure);

    ScoredVariant {
        variant_id: variant.id.clone(),
        score: final_score,
        trait_fit: final_trait_fit,
        channel_fit: final_channel_fit,
        trust: final_trust,
        objection_pressure: final_objection_pressure,
        funnel: build_funnel(
            final_score,
            final_trait_fit,
            final_channel_fit,
            final_trust,
            final_objection_pressure,
            trait_value(buyer, "social_sharing"),
            timeline_time_factor(scenario),
            stable_seed(seed_base, &buyer.id.0, &variant.id),
        ),
        touchpoints,
    }
}

fn base_components(
    buyer: &Buyer,
    segment: &ConceptSegment,
    variant: &ConceptVariant,
    scenario: &ConceptScenario,
) -> BuyerVariantComponents {
    let segment_fit = if variant.target_segments.is_empty() {
        0.62
    } else if variant
        .target_segments
        .iter()
        .any(|segment_id| segment_id == &buyer.segment_id)
    {
        1.0
    } else {
        0.32
    };
    BuyerVariantComponents {
        segment_fit,
        trait_fit: trait_fit_score(&buyer.traits, variant),
        channel_fit: channel_fit_score(buyer, segment, variant, scenario),
        trust: trust_score(buyer, segment, variant),
        objection_pressure: objection_pressure_score(buyer, segment, variant),
        time_factor: (scenario.time_steps as f64 / default_time_steps() as f64).clamp(0.65, 1.35),
    }
}

fn weighted_component_score(components: &BuyerVariantComponents) -> f64 {
    let weighted = (components.segment_fit * 0.24)
        + (components.trait_fit * 0.27)
        + (components.channel_fit * 0.18)
        + (components.trust * 0.20)
        + ((1.0 - components.objection_pressure) * 0.11);
    let dimensions = [
        components.segment_fit,
        components.trait_fit,
        components.channel_fit,
        components.trust,
        1.0 - components.objection_pressure,
    ];
    let min_factor = dimensions
        .iter()
        .copied()
        .fold(1.0_f64, f64::min)
        .clamp(0.0, 1.0);
    let floor = 0.65 + 0.35 * min_factor;
    clamp01(weighted * floor)
}

fn score_touchpoint(
    buyer: &Buyer,
    segment: &ConceptSegment,
    variant: &ConceptVariant,
    scenario: &ConceptScenario,
    touchpoint: &ConceptTouchpoint,
    index: usize,
    components: BuyerVariantComponents,
    prior_score: f64,
    trust_memory: f64,
    objection_memory: f64,
    seed_base: u64,
) -> ScoredTouchpoint {
    let intensity = touchpoint.intensity.clamp(0.0, 2.0);
    let variant_alignment = if touchpoint.variants.is_empty()
        || touchpoint
            .variants
            .iter()
            .any(|variant_id| variant_id == &variant.id)
    {
        0.09
    } else {
        -0.14
    };
    let segment_alignment = if touchpoint.target_segments.is_empty() {
        0.0
    } else if touchpoint
        .target_segments
        .iter()
        .any(|segment_id| segment_id == &buyer.segment_id)
    {
        0.11
    } else {
        -0.08
    };
    let touchpoint_trait_fit = weighted_trait_fit(&buyer.traits, &touchpoint.trait_weights)
        .unwrap_or(components.trait_fit);
    let touchpoint_channel_fit = touchpoint_channel_fit(
        buyer,
        segment,
        variant,
        scenario,
        touchpoint,
        components.channel_fit,
    );
    let mut touchpoint_trust = clamp01(
        (trust_memory * 0.55)
            + (components.trust * 0.25)
            + (touchpoint_trait_fit * 0.08)
            + (focus_trust_shift(touchpoint.focus, buyer, variant) * intensity),
    );
    let mut touchpoint_objection = clamp01(
        (objection_memory * 0.55)
            + (components.objection_pressure * 0.25)
            + (focus_objection_shift(touchpoint.focus, segment, variant) * intensity),
    );
    if touchpoint.focus == ConceptTouchpointFocus::Stressor {
        touchpoint_trust = clamp01(touchpoint_trust - (0.08 * intensity));
        touchpoint_objection = clamp01(touchpoint_objection + (0.14 * intensity));
    }

    let focus_score = focus_score_shift(
        touchpoint.focus,
        buyer,
        touchpoint_trait_fit,
        touchpoint_channel_fit,
        touchpoint_trust,
        touchpoint_objection,
    );
    let step_score = clamp01(
        (prior_score * 0.36)
            + (components.segment_fit * 0.12)
            + (touchpoint_trait_fit * 0.18)
            + (touchpoint_channel_fit * 0.15)
            + (touchpoint_trust * 0.14)
            + ((1.0 - touchpoint_objection) * 0.05)
            + ((variant_alignment + segment_alignment + focus_score) * intensity),
    );

    let touchpoint_id = touchpoint_key(index, touchpoint);
    ScoredTouchpoint {
        score: step_score,
        trait_fit: touchpoint_trait_fit,
        channel_fit: touchpoint_channel_fit,
        trust: touchpoint_trust,
        objection_pressure: touchpoint_objection,
        funnel: build_funnel(
            step_score,
            touchpoint_trait_fit,
            touchpoint_channel_fit,
            touchpoint_trust,
            touchpoint_objection,
            trait_value(buyer, "social_sharing"),
            touchpoint_time_factor(scenario, touchpoint),
            stable_seed_with_scope(seed_base, &buyer.id.0, &variant.id, &touchpoint_id),
        ),
    }
}

fn build_funnel(
    score: f64,
    trait_fit: f64,
    channel_fit: f64,
    trust: f64,
    objection_pressure: f64,
    social_sharing: f64,
    time_factor: f64,
    seed: u64,
) -> IndividualFunnel {
    let click_probability =
        clamp01(0.08 + (score * 0.46) + (channel_fit * 0.24) + (time_factor * 0.04));
    let signup_probability = clamp01(
        click_probability * (0.18 + (score * 0.52) + (trust * 0.20) - (objection_pressure * 0.16)),
    );
    let activation_probability = clamp01(
        signup_probability
            * (0.24 + (trait_fit * 0.34) + (trust * 0.22) - (objection_pressure * 0.12)),
    );
    let retention_probability = clamp01(
        activation_probability
            * (0.30 + (trust * 0.27) + (score * 0.20) - (objection_pressure * 0.08)),
    );
    let referral_probability =
        clamp01(retention_probability * (0.12 + (channel_fit * 0.16) + (social_sharing * 0.12)));

    let mut rng = StdRng::seed_from_u64(seed);
    let clicked = rng.gen_bool(click_probability);
    let signed_up = clicked && rng.gen_bool(signup_probability);
    let activated = signed_up && rng.gen_bool(activation_probability);
    let retained = activated && rng.gen_bool(retention_probability);
    let referred = retained && rng.gen_bool(referral_probability);

    IndividualFunnel {
        click_probability,
        signup_probability,
        activation_probability,
        retention_probability,
        referral_probability,
        clicked,
        signed_up,
        activated,
        retained,
        referred,
    }
}

fn touchpoint_average(
    touchpoints: &[ScoredTouchpoint],
    value_fn: impl Fn(&ScoredTouchpoint) -> f64,
) -> Option<f64> {
    if touchpoints.is_empty() {
        return None;
    }
    Some(touchpoints.iter().map(value_fn).sum::<f64>() / touchpoints.len() as f64)
}

fn touchpoint_channel_fit(
    buyer: &Buyer,
    segment: &ConceptSegment,
    variant: &ConceptVariant,
    scenario: &ConceptScenario,
    touchpoint: &ConceptTouchpoint,
    base_channel_fit: f64,
) -> f64 {
    let Some(channel) = touchpoint.channel.as_deref() else {
        return base_channel_fit;
    };

    let buyer_match = strings_match(channel, &buyer.primary_channel);
    let segment_match = segment
        .channels
        .iter()
        .any(|segment_channel| strings_match(channel, segment_channel));
    let variant_match = variant
        .channels
        .iter()
        .any(|variant_channel| strings_match(channel, variant_channel));
    let scenario_match = scenario
        .channels
        .iter()
        .any(|scenario_channel| strings_match(channel, scenario_channel));

    clamp01(
        (if buyer_match { 0.36 } else { 0.08 })
            + (if segment_match { 0.22 } else { 0.0 })
            + (if variant_match { 0.28 } else { 0.0 })
            + (if scenario_match { 0.14 } else { 0.0 }),
    )
}

fn focus_trust_shift(
    focus: ConceptTouchpointFocus,
    buyer: &Buyer,
    variant: &ConceptVariant,
) -> f64 {
    let variant_text = variant_text(variant);
    match focus {
        ConceptTouchpointFocus::Awareness => 0.01,
        ConceptTouchpointFocus::Resonance => 0.03 + (trait_value(buyer, "proof_hunger") * 0.03),
        ConceptTouchpointFocus::Proof => {
            0.06 + (trait_value(buyer, "proof_hunger") * 0.08)
                + if contains_any(&variant_text, &["proof", "evidence", "case", "study"]) {
                    0.04
                } else {
                    0.0
                }
        }
        ConceptTouchpointFocus::ObjectionHandling => 0.07,
        ConceptTouchpointFocus::Conversion => 0.02,
        ConceptTouchpointFocus::Retention => 0.06,
        ConceptTouchpointFocus::Referral => 0.03,
        ConceptTouchpointFocus::Stressor => -0.05,
    }
}

fn focus_objection_shift(
    focus: ConceptTouchpointFocus,
    segment: &ConceptSegment,
    variant: &ConceptVariant,
) -> f64 {
    let variant_text = variant_text(variant);
    let handles_segment_objection = segment
        .objections
        .iter()
        .any(|objection| variant_text.contains(&objection.to_ascii_lowercase()));
    match focus {
        ConceptTouchpointFocus::Awareness => 0.01,
        ConceptTouchpointFocus::Resonance => -0.01,
        ConceptTouchpointFocus::Proof => -0.04,
        ConceptTouchpointFocus::ObjectionHandling => {
            if handles_segment_objection {
                -0.14
            } else {
                -0.08
            }
        }
        ConceptTouchpointFocus::Conversion => 0.03,
        ConceptTouchpointFocus::Retention => -0.03,
        ConceptTouchpointFocus::Referral => -0.01,
        ConceptTouchpointFocus::Stressor => 0.12,
    }
}

fn focus_score_shift(
    focus: ConceptTouchpointFocus,
    buyer: &Buyer,
    trait_fit: f64,
    channel_fit: f64,
    trust: f64,
    objection_pressure: f64,
) -> f64 {
    match focus {
        ConceptTouchpointFocus::Awareness => (channel_fit * 0.08) + 0.03,
        ConceptTouchpointFocus::Resonance => (trait_fit * 0.08) + (trust * 0.05),
        ConceptTouchpointFocus::Proof => {
            (trust * 0.08) + (trait_value(buyer, "proof_hunger") * 0.07)
        }
        ConceptTouchpointFocus::ObjectionHandling => (1.0 - objection_pressure) * 0.12,
        ConceptTouchpointFocus::Conversion => (channel_fit * 0.05) + (trust * 0.05),
        ConceptTouchpointFocus::Retention => (trust * 0.08) + (trait_fit * 0.04),
        ConceptTouchpointFocus::Referral => {
            (trait_value(buyer, "social_sharing") * 0.11) + (channel_fit * 0.03)
        }
        ConceptTouchpointFocus::Stressor => -0.10,
    }
}

fn timeline_time_factor(scenario: &ConceptScenario) -> f64 {
    let base = (scenario.time_steps as f64 / default_time_steps() as f64).clamp(0.65, 1.35);
    if scenario.touchpoints.is_empty() {
        base
    } else {
        let sequence_strength = scenario
            .touchpoints
            .iter()
            .map(|touchpoint| touchpoint.intensity.clamp(0.0, 2.0))
            .sum::<f64>()
            / (scenario.touchpoints.len() as f64 * 1.25);
        (base + (sequence_strength.clamp(0.0, 1.0) * 0.08)).clamp(0.65, 1.40)
    }
}

fn touchpoint_time_factor(scenario: &ConceptScenario, touchpoint: &ConceptTouchpoint) -> f64 {
    let base = (scenario.time_steps as f64 / default_time_steps() as f64).clamp(0.65, 1.35);
    (base + (touchpoint.intensity.clamp(0.0, 2.0) * 0.04)).clamp(0.65, 1.40)
}

fn touchpoint_key(index: usize, touchpoint: &ConceptTouchpoint) -> String {
    touchpoint
        .id
        .as_deref()
        .filter(|id| !id.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("touchpoint-{}", index + 1))
}

fn trait_fit_score(traits: &BTreeMap<String, f64>, variant: &ConceptVariant) -> f64 {
    weighted_trait_fit(traits, &variant.trait_weights).unwrap_or(0.55)
}

fn weighted_trait_fit(
    traits: &BTreeMap<String, f64>,
    weights: &BTreeMap<String, f64>,
) -> Option<f64> {
    if weights.is_empty() {
        return None;
    }
    let mut weighted_sum = 0.0;
    let mut total_weight = 0.0;
    for (trait_name, weight) in weights {
        let value = traits
            .get(trait_name)
            .copied()
            .unwrap_or(0.5)
            .clamp(0.0, 1.0);
        let directional = if *weight >= 0.0 { value } else { 1.0 - value };
        let weight_abs = weight.abs();
        weighted_sum += directional * weight_abs;
        total_weight += weight_abs;
    }

    if total_weight <= f64::EPSILON {
        None
    } else {
        Some(clamp01(weighted_sum / total_weight))
    }
}

fn channel_fit_score(
    buyer: &Buyer,
    segment: &ConceptSegment,
    variant: &ConceptVariant,
    scenario: &ConceptScenario,
) -> f64 {
    if variant.channels.is_empty() {
        return 0.50;
    }

    let buyer_channel_match = variant
        .channels
        .iter()
        .any(|channel| strings_match(channel, &buyer.primary_channel));
    let segment_overlap = overlap_ratio(&variant.channels, &segment.channels);
    let scenario_overlap = overlap_ratio(&variant.channels, &scenario.channels);

    clamp01(
        (if buyer_channel_match { 0.42 } else { 0.10 })
            + (segment_overlap * 0.34)
            + (scenario_overlap * 0.24),
    )
}

fn trust_score(buyer: &Buyer, segment: &ConceptSegment, variant: &ConceptVariant) -> f64 {
    let strength_text = variant
        .strengths
        .iter()
        .chain(variant.summary.iter())
        .map(|value| value.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(" ");
    let proof_bonus = if contains_any(&strength_text, &["proof", "evidence", "case", "study"]) {
        trait_value(buyer, "proof_hunger") * 0.16
    } else {
        0.0
    };
    let privacy_bonus = if contains_any(&strength_text, &["privacy", "private", "secure"]) {
        trait_value(buyer, "privacy_sensitivity") * 0.12
    } else {
        0.0
    };
    let objection_bonus = if segment
        .objections
        .iter()
        .any(|objection| strength_text.contains(&objection.to_ascii_lowercase()))
    {
        0.08
    } else {
        0.0
    };

    clamp01((buyer.trust * 0.64) + 0.18 + proof_bonus + privacy_bonus + objection_bonus)
}

fn objection_pressure_score(
    buyer: &Buyer,
    segment: &ConceptSegment,
    variant: &ConceptVariant,
) -> f64 {
    let base = if buyer.objections.is_empty() {
        0.10
    } else {
        buyer
            .objections
            .iter()
            .map(|objection| objection.severity)
            .sum::<f64>()
            / buyer.objections.len() as f64
    };
    let variant_text = variant
        .strengths
        .iter()
        .chain(variant.summary.iter())
        .map(|value| value.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(" ");
    let mitigation = segment
        .objections
        .iter()
        .filter(|objection| variant_text.contains(&objection.to_ascii_lowercase()))
        .count() as f64
        * 0.08;
    let risk_penalty = variant.risks.len().min(4) as f64 * 0.035;

    clamp01(base + risk_penalty - mitigation)
}

fn variant_text(variant: &ConceptVariant) -> String {
    variant
        .strengths
        .iter()
        .chain(variant.risks.iter())
        .chain(variant.summary.iter())
        .map(|value| value.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(" ")
}

fn trait_value(buyer: &Buyer, trait_name: &str) -> f64 {
    buyer
        .traits
        .get(trait_name)
        .copied()
        .unwrap_or(0.5)
        .clamp(0.0, 1.0)
}

fn overlap_ratio(left: &[String], right: &[String]) -> f64 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    let matches = left
        .iter()
        .filter(|left_value| {
            right
                .iter()
                .any(|right_value| strings_match(left_value, right_value))
        })
        .count();
    matches as f64 / left.len() as f64
}

fn strings_match(left: &str, right: &str) -> bool {
    left.trim().eq_ignore_ascii_case(right.trim())
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

impl ConceptAccumulator {
    fn add(&mut self, score: &ScoredVariant) {
        self.buyers += 1;
        self.score_sum += score.score;
        self.score_sum_sq += score.score * score.score;
        self.trait_fit_sum += score.trait_fit;
        self.channel_fit_sum += score.channel_fit;
        self.trust_sum += score.trust;
        self.objection_pressure_sum += score.objection_pressure;
        self.clicks += usize::from(score.funnel.clicked);
        self.signups += usize::from(score.funnel.signed_up);
        self.activations += usize::from(score.funnel.activated);
        self.retained += usize::from(score.funnel.retained);
        self.referrals += usize::from(score.funnel.referred);
    }

    fn add_touchpoint(&mut self, score: &ScoredTouchpoint) {
        self.buyers += 1;
        self.score_sum += score.score;
        self.score_sum_sq += score.score * score.score;
        self.trait_fit_sum += score.trait_fit;
        self.channel_fit_sum += score.channel_fit;
        self.trust_sum += score.trust;
        self.objection_pressure_sum += score.objection_pressure;
        self.clicks += usize::from(score.funnel.clicked);
        self.signups += usize::from(score.funnel.signed_up);
        self.activations += usize::from(score.funnel.activated);
        self.retained += usize::from(score.funnel.retained);
        self.referrals += usize::from(score.funnel.referred);
    }

    fn score_interval(&self) -> ConceptScoreInterval {
        if self.buyers == 0 {
            return ConceptScoreInterval::default();
        }
        let n = self.buyers as f64;
        let mean = (self.score_sum / n).clamp(0.0, 1.0);
        let stddev = if self.buyers >= 2 {
            let variance = ((self.score_sum_sq / n) - mean * mean).max(0.0) * n / (n - 1.0);
            variance.sqrt()
        } else {
            0.0
        };
        let half_width = if self.buyers >= 2 {
            1.96 * (stddev / n.sqrt())
        } else {
            0.0
        };
        ConceptScoreInterval {
            mean: as_percent(mean),
            lower: as_percent((mean - half_width).max(0.0)),
            upper: as_percent((mean + half_width).min(1.0)),
            stddev,
            sample_size: self.buyers,
        }
    }

    fn average_score(&self) -> u32 {
        if self.buyers == 0 {
            0
        } else {
            as_percent(self.score_sum / self.buyers as f64)
        }
    }

    fn average_trait_fit(&self) -> u32 {
        self.average_component(self.trait_fit_sum)
    }

    fn average_channel_fit(&self) -> u32 {
        self.average_component(self.channel_fit_sum)
    }

    fn average_trust(&self) -> u32 {
        self.average_component(self.trust_sum)
    }

    fn average_objection_pressure(&self) -> u32 {
        self.average_component(self.objection_pressure_sum)
    }

    fn average_component(&self, sum: f64) -> u32 {
        if self.buyers == 0 {
            0
        } else {
            as_percent(sum / self.buyers as f64)
        }
    }

    fn funnel(&self) -> ConceptFunnelMetrics {
        let buyers = self.buyers.max(1) as f64;
        ConceptFunnelMetrics {
            buyers: self.buyers,
            clicks: self.clicks,
            signups: self.signups,
            activations: self.activations,
            retained: self.retained,
            referrals: self.referrals,
            click_rate: self.clicks as f64 / buyers,
            signup_rate: self.signups as f64 / buyers,
            activation_rate: self.activations as f64 / buyers,
            retention_rate: self.retained as f64 / buyers,
            referral_rate: self.referrals as f64 / buyers,
        }
    }
}

fn build_segment_result(
    segment: &ConceptSegment,
    accumulator: &ConceptAccumulator,
) -> ConceptSegmentResult {
    ConceptSegmentResult {
        segment_id: segment.id.clone(),
        segment_name: segment.name.clone(),
        buyers: accumulator.buyers,
        score: accumulator.average_score(),
        funnel: accumulator.funnel(),
        trait_fit_score: accumulator.average_trait_fit(),
        channel_fit_score: accumulator.average_channel_fit(),
        trust_score: accumulator.average_trust(),
        objection_pressure: accumulator.average_objection_pressure(),
    }
}

fn build_touchpoint_results(
    touchpoints: &[ConceptTouchpoint],
    accumulators: &[ConceptAccumulator],
) -> Vec<ConceptTouchpointResult> {
    let mut previous_score = None;
    touchpoints
        .iter()
        .enumerate()
        .filter_map(|(index, touchpoint)| {
            let accumulator = accumulators.get(index)?;
            let score = accumulator.average_score();
            let lift_vs_previous_score = previous_score.map(|previous| score as i32 - previous);
            previous_score = Some(score as i32);
            Some(ConceptTouchpointResult {
                index,
                touchpoint_id: touchpoint_key(index, touchpoint),
                label: touchpoint.label.clone(),
                focus: touchpoint.focus,
                channel: touchpoint.channel.clone(),
                buyers: accumulator.buyers,
                score,
                lift_vs_previous_score,
                funnel: accumulator.funnel(),
                trait_fit_score: accumulator.average_trait_fit(),
                channel_fit_score: accumulator.average_channel_fit(),
                trust_score: accumulator.average_trust(),
                objection_pressure: accumulator.average_objection_pressure(),
            })
        })
        .collect()
}

fn build_segment_summaries(
    segments: &[ConceptSegment],
    segment_variant_accumulators: &BTreeMap<String, BTreeMap<String, ConceptAccumulator>>,
    variants_by_id: &BTreeMap<&str, &ConceptVariant>,
) -> Vec<ConceptSegmentSummary> {
    let mut summaries = Vec::new();
    for segment in segments {
        let Some(by_variant) = segment_variant_accumulators.get(&segment.id) else {
            continue;
        };
        let mut ranked = by_variant
            .iter()
            .filter(|(variant_id, _)| variants_by_id.contains_key(variant_id.as_str()))
            .map(|(variant_id, accumulator)| {
                (
                    variant_id.clone(),
                    accumulator.average_score(),
                    accumulator.buyers,
                )
            })
            .collect::<Vec<_>>();
        ranked.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
        if let Some(best) = ranked.first() {
            let runner_up = ranked.get(1);
            summaries.push(ConceptSegmentSummary {
                segment_id: segment.id.clone(),
                segment_name: segment.name.clone(),
                buyers_simulated: best.2,
                best_variant_id: best.0.clone(),
                best_score: best.1,
                runner_up_variant_id: runner_up.map(|value| value.0.clone()),
                runner_up_score: runner_up.map(|value| value.1),
            });
        }
    }
    summaries.sort_by(|left, right| {
        right
            .best_score
            .cmp(&left.best_score)
            .then_with(|| left.segment_id.cmp(&right.segment_id))
    });
    summaries
}

fn build_calibration_summary(
    observed_outcomes: &[ConceptObservedOutcome],
    ranked_variants: &[ConceptVariantResult],
) -> Vec<ConceptCalibrationSummary> {
    let mut summaries = Vec::new();
    for variant in ranked_variants {
        let matching = observed_outcomes
            .iter()
            .filter(|outcome| {
                outcome.variant_id == variant.variant_id && outcome.touchpoint_id.is_none()
            })
            .collect::<Vec<_>>();
        if matching.is_empty() {
            continue;
        }
        summaries.push(build_calibration_summary_for_funnel(
            &variant.variant_id,
            None,
            &matching,
            &variant.funnel,
        ));
    }
    summaries
}

fn build_touchpoint_calibration_summary(
    observed_outcomes: &[ConceptObservedOutcome],
    ranked_variants: &[ConceptVariantResult],
) -> Vec<ConceptCalibrationSummary> {
    let mut summaries = Vec::new();
    for variant in ranked_variants {
        for touchpoint in &variant.touchpoint_results {
            let matching = observed_outcomes
                .iter()
                .filter(|outcome| {
                    outcome.variant_id == variant.variant_id
                        && outcome.touchpoint_id.as_deref()
                            == Some(touchpoint.touchpoint_id.as_str())
                })
                .collect::<Vec<_>>();
            if matching.is_empty() {
                continue;
            }
            summaries.push(build_calibration_summary_for_funnel(
                &variant.variant_id,
                Some(touchpoint.touchpoint_id.as_str()),
                &matching,
                &touchpoint.funnel,
            ));
        }
    }
    summaries
}

fn build_calibration_summary_for_funnel(
    variant_id: &str,
    touchpoint_id: Option<&str>,
    matching: &[&ConceptObservedOutcome],
    funnel: &ConceptFunnelMetrics,
) -> ConceptCalibrationSummary {
    let usable = matching
        .iter()
        .copied()
        .filter(|outcome| observed_outcome_is_usable(outcome))
        .collect::<Vec<_>>();
    let observed_sample_size = sum_sample_size(usable.iter().map(|outcome| outcome.sample_size));
    let click_gap = weighted_gap(&usable, |outcome| outcome.click_rate, funnel.click_rate);
    let signup_gap = weighted_gap(&usable, |outcome| outcome.signup_rate, funnel.signup_rate);
    let activation_gap = weighted_gap(
        &usable,
        |outcome| outcome.activation_rate,
        funnel.activation_rate,
    );
    let retention_gap = weighted_gap(
        &usable,
        |outcome| outcome.retention_rate,
        funnel.retention_rate,
    );
    let referral_gap = weighted_gap(
        &usable,
        |outcome| outcome.referral_rate,
        funnel.referral_rate,
    );
    let compared_metrics = compared_metric_names(
        click_gap,
        signup_gap,
        activation_gap,
        retention_gap,
        referral_gap,
    );

    ConceptCalibrationSummary {
        variant_id: variant_id.to_string(),
        touchpoint_id: touchpoint_id.map(str::to_string),
        observed_records: matching.len(),
        usable_observed_records: usable.len(),
        observed_sample_size,
        compared_metrics,
        click_gap,
        signup_gap,
        activation_gap,
        retention_gap,
        referral_gap,
        note: calibration_note(
            matching.len(),
            usable.len(),
            observed_sample_size,
            click_gap,
            signup_gap,
            activation_gap,
            retention_gap,
            referral_gap,
        ),
    }
}

fn observed_outcome_is_usable(outcome: &ConceptObservedOutcome) -> bool {
    outcome.click_rate.is_some()
        || outcome.signup_rate.is_some()
        || outcome.activation_rate.is_some()
        || outcome.retention_rate.is_some()
        || outcome.referral_rate.is_some()
}

fn weighted_gap(
    outcomes: &[&ConceptObservedOutcome],
    metric_fn: impl Fn(&ConceptObservedOutcome) -> Option<f64>,
    simulated_value: f64,
) -> Option<f64> {
    let mut weighted_sum = 0.0;
    let mut total_weight = 0.0;
    for outcome in outcomes {
        let Some(value) = metric_fn(outcome) else {
            continue;
        };
        let weight = positive_sample_size(outcome.sample_size)
            .map(|value| value as f64)
            .unwrap_or(1.0);
        weighted_sum += value * weight;
        total_weight += weight;
    }
    if total_weight <= f64::EPSILON {
        None
    } else {
        Some(simulated_value - (weighted_sum / total_weight))
    }
}

fn compared_metric_names(
    click_gap: Option<f64>,
    signup_gap: Option<f64>,
    activation_gap: Option<f64>,
    retention_gap: Option<f64>,
    referral_gap: Option<f64>,
) -> Vec<String> {
    [
        ("click_rate", click_gap),
        ("signup_rate", signup_gap),
        ("activation_rate", activation_gap),
        ("retention_rate", retention_gap),
        ("referral_rate", referral_gap),
    ]
    .into_iter()
    .filter_map(|(label, gap)| gap.map(|_| label.to_string()))
    .collect()
}

fn calibration_note(
    observed_records: usize,
    usable_records: usize,
    sample_size: Option<u32>,
    click_gap: Option<f64>,
    signup_gap: Option<f64>,
    activation_gap: Option<f64>,
    retention_gap: Option<f64>,
    referral_gap: Option<f64>,
) -> String {
    if observed_records == 0 {
        return "No observed outcomes matched this variant yet.".into();
    }
    if usable_records == 0 {
        return "Observed records exist, but no comparable rates were populated.".into();
    }

    let mut parts = Vec::new();
    for (label, gap) in [
        ("click", click_gap),
        ("signup", signup_gap),
        ("activation", activation_gap),
        ("retention", retention_gap),
        ("referral", referral_gap),
    ] {
        if let Some(gap) = gap {
            let direction = if gap > 0.02 {
                "over"
            } else if gap < -0.02 {
                "under"
            } else {
                "near"
            };
            parts.push(format!("{label}:{direction}"));
        }
    }

    let confidence = match sample_size.unwrap_or_default() {
        0..=99 => "low",
        100..=499 => "medium",
        _ => "high",
    };
    format!(
        "Calibration signal {} (confidence:{confidence})",
        parts.join(", ")
    )
}

fn build_notes(
    request: &ConceptTestRequest,
    ranked_variants: &[ConceptVariantResult],
    calibration_summary: &[ConceptCalibrationSummary],
    touchpoint_calibration_summary: &[ConceptCalibrationSummary],
    recommendation_status: &ConceptRecommendationStatus,
) -> Vec<String> {
    let mut notes = Vec::new();
    match recommendation_status {
        ConceptRecommendationStatus::Uncalibrated => {
            notes.push("No usable observed outcomes were attached, so this run is directional only — use it to choose the next real-world test, not as a forecast.".into());
        }
        ConceptRecommendationStatus::TiedWithinNoise => {
            notes.push(
                "Top-ranked and runner-up score 95% confidence intervals overlap. Treat this as tied within noise — pick the next real-world test instead of declaring a winner."
                    .into(),
            );
        }
        ConceptRecommendationStatus::CalibratedRecommended => {}
    }
    if !touchpoint_calibration_summary.is_empty() {
        notes.push(format!(
            "Touchpoint calibration is active for `{}` step-level outcome summaries.",
            touchpoint_calibration_summary.len()
        ));
    } else if !request.scenario.touchpoints.is_empty() && !calibration_summary.is_empty() {
        notes.push(
            "Only aggregate observed outcomes were attached; add touchpoint_id to calibrate timeline steps."
                .into(),
        );
    }
    if let Some(winner) = ranked_variants.first() {
        let label = match recommendation_status {
            ConceptRecommendationStatus::CalibratedRecommended => "is the recommended control",
            _ => "is the current top-ranked variant",
        };
        notes.push(format!(
            "`{}` {label} with overall score `{}` (95% CI {}-{}).",
            winner.variant_id,
            winner.overall_score,
            winner.score_interval.lower,
            winner.score_interval.upper
        ));
    }
    if let (Some(winner), Some(challenger)) = (ranked_variants.first(), ranked_variants.get(1)) {
        let gap = winner.overall_score as i32 - challenger.overall_score as i32;
        notes.push(format!(
            "Runner-up `{}` scored `{}` (95% CI {}-{}); top-vs-runner-up gap is `{gap}` points.",
            challenger.variant_id,
            challenger.overall_score,
            challenger.score_interval.lower,
            challenger.score_interval.upper
        ));
    }
    if request.population.target_count < 1_000 {
        notes.push(
            "Population size is below 1,000, so use this mostly for smoke testing scenario shape."
                .into(),
        );
    }
    notes
}

fn positive_sample_size(sample_size: Option<u32>) -> Option<u32> {
    sample_size.filter(|value| *value > 0)
}

fn sum_sample_size(sample_sizes: impl Iterator<Item = Option<u32>>) -> Option<u32> {
    let mut total = 0u32;
    let mut has_value = false;
    for sample_size in sample_sizes {
        if let Some(value) = positive_sample_size(sample_size) {
            total = total.saturating_add(value);
            has_value = true;
        }
    }
    has_value.then_some(total)
}

fn stable_seed(seed_base: u64, buyer_id: &str, variant_id: &str) -> u64 {
    let mut hasher = stable_hasher("concept_buyer_variant_seed_v1");
    update_hash_u64(&mut hasher, seed_base);
    update_hash_str(&mut hasher, buyer_id);
    update_hash_str(&mut hasher, variant_id);
    finish_hash_u64(hasher)
}

fn stable_seed_with_scope(seed_base: u64, buyer_id: &str, variant_id: &str, scope: &str) -> u64 {
    let mut hasher = stable_hasher("concept_scoped_buyer_variant_seed_v1");
    update_hash_u64(&mut hasher, seed_base);
    update_hash_str(&mut hasher, buyer_id);
    update_hash_str(&mut hasher, variant_id);
    update_hash_str(&mut hasher, scope);
    finish_hash_u64(hasher)
}

fn stable_hasher(scope: &str) -> Sha256 {
    let mut hasher = Sha256::new();
    update_hash_str(&mut hasher, scope);
    hasher
}

fn update_hash_str(hasher: &mut Sha256, value: &str) {
    hasher.update((value.len() as u64).to_le_bytes());
    hasher.update(value.as_bytes());
}

fn update_hash_u64(hasher: &mut Sha256, value: u64) {
    hasher.update(value.to_le_bytes());
}

fn finish_hash_u64(hasher: Sha256) -> u64 {
    let digest = hasher.finalize();
    let mut bytes = [0_u8; 8];
    bytes.copy_from_slice(&digest[..8]);
    u64::from_le_bytes(bytes)
}

fn as_percent(value: f64) -> u32 {
    (clamp01(value) * 100.0).round() as u32
}

fn clamp01(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn default_seed_base() -> u64 {
    42
}

fn default_population_size() -> usize {
    10_000
}

fn default_sample_size() -> usize {
    24
}

fn default_share_weight() -> f64 {
    1.0
}

fn default_time_steps() -> usize {
    8
}

fn default_touchpoint_intensity() -> f64 {
    1.0
}

#[derive(Debug, Error)]
pub enum ConceptTestError {
    #[error("concept test ID cannot be empty")]
    EmptyId,
    #[error("concept test name cannot be empty")]
    EmptyName,
    #[error("scenario ID cannot be empty")]
    EmptyScenarioId,
    #[error("scenario name cannot be empty")]
    EmptyScenarioName,
    #[error("population target_count must be greater than zero")]
    ZeroPopulation,
    #[error("scenario time_steps must be greater than zero")]
    ZeroTimeSteps,
    #[error("at least one segment is required")]
    NoSegments,
    #[error("at least one variant is required")]
    NoVariants,
    #[error("segment ID cannot be empty")]
    EmptySegmentId,
    #[error("segment name cannot be empty")]
    EmptySegmentName,
    #[error("duplicate segment ID `{0}`")]
    DuplicateSegmentId(String),
    #[error("segment `{segment_id}` share_weight must be positive and finite, got {value}")]
    InvalidSegmentShare { segment_id: String, value: f64 },
    #[error("segment `{0}` target_count must be greater than zero when provided")]
    InvalidSegmentTargetCount(String),
    #[error("segment `{0}` has an empty trait name")]
    InvalidTraitName(String),
    #[error("segment `{segment_id}` trait `{trait_name}` must be in [0, 1], got {value}")]
    InvalidTraitValue {
        segment_id: String,
        trait_name: String,
        value: f64,
    },
    #[error("variant ID cannot be empty")]
    EmptyVariantId,
    #[error("variant name cannot be empty")]
    EmptyVariantName,
    #[error("duplicate variant ID `{0}`")]
    DuplicateVariantId(String),
    #[error("variant `{variant_id}` targets unknown segment `{segment_id}`")]
    UnknownTargetSegment {
        variant_id: String,
        segment_id: String,
    },
    #[error("variant `{0}` has an empty trait weight name")]
    InvalidVariantTraitName(String),
    #[error("variant `{variant_id}` trait weight `{trait_name}` must be finite, got {value}")]
    InvalidVariantTraitWeight {
        variant_id: String,
        trait_name: String,
        value: f64,
    },
    #[error("touchpoint ID cannot be empty when provided")]
    EmptyTouchpointId,
    #[error("duplicate touchpoint ID `{0}`")]
    DuplicateTouchpointId(String),
    #[error("touchpoint label cannot be empty")]
    EmptyTouchpointLabel,
    #[error("touchpoint channel cannot be empty when provided")]
    EmptyTouchpointChannel,
    #[error("touchpoint `{label}` intensity must be finite and non-negative, got {value}")]
    InvalidTouchpointIntensity { label: String, value: f64 },
    #[error("touchpoint `{label}` references unknown segment `{segment_id}`")]
    UnknownTouchpointSegment { label: String, segment_id: String },
    #[error("touchpoint `{label}` references unknown variant `{variant_id}`")]
    UnknownTouchpointVariant { label: String, variant_id: String },
    #[error("touchpoint `{0}` has an empty trait weight name")]
    InvalidTouchpointTraitName(String),
    #[error("touchpoint `{label}` trait weight `{trait_name}` must be finite, got {value}")]
    InvalidTouchpointTraitWeight {
        label: String,
        trait_name: String,
        value: f64,
    },
    #[error("observed outcome references unknown variant `{0}`")]
    UnknownObservedVariant(String),
    #[error("observed outcome references unknown segment `{0}`")]
    UnknownObservedSegment(String),
    #[error("observed outcome for variant `{variant_id}` references unknown touchpoint `{touchpoint_id}`")]
    UnknownObservedTouchpoint {
        variant_id: String,
        touchpoint_id: String,
    },
    #[error("observed outcome for variant `{variant_id}` has invalid {field}: {value}")]
    InvalidObservedRate {
        variant_id: String,
        field: &'static str,
        value: f64,
    },
    #[error("generated buyer referenced missing segment `{0}`")]
    MissingGeneratedSegment(String),
    #[error("population generation error: {0}")]
    Population(composure_population::population::PopError),
}

#[derive(Debug, Error)]
pub enum ConceptTestCompareError {
    #[error("at least two concept test results are required for comparison")]
    TooFewInputs,
    #[error("concept test result `{0}` has no ranked variants")]
    EmptyResult(String),
}

#[derive(Debug, Error)]
pub enum ConceptTestMatrixError {
    #[error("matrix ID cannot be empty")]
    EmptyId,
    #[error("matrix name cannot be empty")]
    EmptyName,
    #[error("at least one matrix case is required")]
    NoCases,
    #[error("matrix case ID cannot be empty")]
    EmptyCaseId,
    #[error("matrix case name cannot be empty")]
    EmptyCaseName,
    #[error("duplicate matrix case ID `{0}`")]
    DuplicateCaseId(String),
    #[error("matrix case `{0}` must provide either scenario or preset")]
    MissingCaseScenario(String),
    #[error("matrix case `{0}` returned no ranked variants")]
    EmptyCaseResult(String),
    #[error("matrix case `{case_id}` failed: {source}")]
    CaseSimulation {
        case_id: String,
        source: ConceptTestError,
    },
}

#[derive(Debug, Error)]
pub enum ConceptTestMatrixDraftError {
    #[error("draft prompt cannot be empty")]
    EmptyPrompt,
    #[error("case_count must be greater than zero, got {0}")]
    InvalidCaseCount(usize),
    #[error("drafted base concept test is invalid: {0}")]
    ConceptTest(ConceptTestError),
    #[error("drafted matrix is invalid: {0}")]
    Matrix(ConceptTestMatrixError),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_request() -> ConceptTestRequest {
        ConceptTestRequest {
            id: "selfrx-wedge".into(),
            name: "SelfRX Wedge Test".into(),
            description: None,
            seed_base: 7,
            population: ConceptPopulationConfig {
                target_count: 1_200,
                sample_size: 8,
            },
            segments: vec![
                ConceptSegment {
                    id: "proof_seekers".into(),
                    name: "Proof Seekers".into(),
                    share_weight: 0.6,
                    traits: BTreeMap::from([
                        ("proof_hunger".into(), 0.88),
                        ("privacy_sensitivity".into(), 0.44),
                        ("social_sharing".into(), 0.42),
                    ]),
                    channels: vec!["landing_page".into(), "reddit".into()],
                    objections: vec!["not enough proof".into()],
                    target_count: None,
                },
                ConceptSegment {
                    id: "privacy_trackers".into(),
                    name: "Privacy Trackers".into(),
                    share_weight: 0.4,
                    traits: BTreeMap::from([
                        ("proof_hunger".into(), 0.52),
                        ("privacy_sensitivity".into(), 0.91),
                        ("social_sharing".into(), 0.22),
                    ]),
                    channels: vec!["reddit".into(), "newsletter".into()],
                    objections: vec!["privacy".into()],
                    target_count: None,
                },
            ],
            variants: vec![
                ConceptVariant {
                    id: "proof_first".into(),
                    name: "Proof First".into(),
                    summary: Some("Weekly proof with confidence scores".into()),
                    target_segments: vec!["proof_seekers".into()],
                    channels: vec!["landing_page".into(), "reddit".into()],
                    trait_weights: BTreeMap::from([("proof_hunger".into(), 1.0)]),
                    strengths: vec!["proof".into(), "case study".into()],
                    risks: vec![],
                },
                ConceptVariant {
                    id: "privacy_first".into(),
                    name: "Privacy First".into(),
                    summary: Some("Private, exportable protocol tracking".into()),
                    target_segments: vec!["privacy_trackers".into()],
                    channels: vec!["reddit".into(), "newsletter".into()],
                    trait_weights: BTreeMap::from([("privacy_sensitivity".into(), 1.0)]),
                    strengths: vec!["privacy".into(), "secure export".into()],
                    risks: vec![],
                },
            ],
            scenario: ConceptScenario {
                id: "wedge".into(),
                name: "Wedge Test".into(),
                goal: Some("Pick an acquisition wedge".into()),
                decision: Some("Choose control and challenger".into()),
                channels: vec!["landing_page".into(), "reddit".into()],
                time_steps: 8,
                success_metrics: vec!["signup_rate".into()],
                touchpoints: vec![
                    ConceptTouchpoint {
                        id: Some("hook".into()),
                        label: "Hook".into(),
                        channel: Some("reddit".into()),
                        focus: ConceptTouchpointFocus::Awareness,
                        intensity: 1.0,
                        target_segments: vec!["proof_seekers".into()],
                        variants: vec![],
                        trait_weights: BTreeMap::new(),
                    },
                    ConceptTouchpoint {
                        id: Some("proof".into()),
                        label: "Proof Follow-up".into(),
                        channel: Some("landing_page".into()),
                        focus: ConceptTouchpointFocus::Proof,
                        intensity: 1.2,
                        target_segments: vec!["proof_seekers".into()],
                        variants: vec!["proof_first".into()],
                        trait_weights: BTreeMap::from([("proof_hunger".into(), 1.0)]),
                    },
                    ConceptTouchpoint {
                        id: Some("objections".into()),
                        label: "Objection Handling".into(),
                        channel: Some("newsletter".into()),
                        focus: ConceptTouchpointFocus::ObjectionHandling,
                        intensity: 0.9,
                        target_segments: vec!["privacy_trackers".into()],
                        variants: vec!["privacy_first".into()],
                        trait_weights: BTreeMap::from([("privacy_sensitivity".into(), 1.0)]),
                    },
                ],
            },
            observed_outcomes: vec![ConceptObservedOutcome {
                variant_id: "proof_first".into(),
                touchpoint_id: None,
                segment_id: None,
                source: Some("waitlist".into()),
                sample_size: Some(120),
                click_rate: Some(0.42),
                signup_rate: Some(0.18),
                activation_rate: None,
                retention_rate: None,
                referral_rate: None,
            }],
        }
    }

    fn sample_matrix_request() -> ConceptTestMatrixRequest {
        let base_request = sample_request();
        let mut trust_case = base_request.scenario.clone();
        trust_case.id = "trust_case".into();
        trust_case.name = "Trust Case".into();
        trust_case.channels = vec!["landing_page".into(), "newsletter".into()];

        let mut referral_case = base_request.scenario.clone();
        referral_case.id = "referral_case".into();
        referral_case.name = "Referral Case".into();
        referral_case.touchpoints.push(ConceptTouchpoint {
            id: Some("referral".into()),
            label: "Referral Prompt".into(),
            channel: Some("reddit".into()),
            focus: ConceptTouchpointFocus::Referral,
            intensity: 1.0,
            target_segments: vec![],
            variants: vec![],
            trait_weights: BTreeMap::from([("social_sharing".into(), 1.0)]),
        });

        ConceptTestMatrixRequest {
            id: "matrix".into(),
            name: "Scenario Matrix".into(),
            description: None,
            base_request,
            cases: vec![
                ConceptTestMatrixCase {
                    id: "trust".into(),
                    name: "Trust Case".into(),
                    description: None,
                    preset: None,
                    scenario: Some(trust_case),
                    seed_base: Some(17),
                    population: Some(ConceptPopulationConfig {
                        target_count: 500,
                        sample_size: 4,
                    }),
                    segments: Vec::new(),
                    observed_outcomes: Vec::new(),
                },
                ConceptTestMatrixCase {
                    id: "referral".into(),
                    name: "Referral Case".into(),
                    description: None,
                    preset: None,
                    scenario: Some(referral_case),
                    seed_base: Some(18),
                    population: Some(ConceptPopulationConfig {
                        target_count: 500,
                        sample_size: 4,
                    }),
                    segments: Vec::new(),
                    observed_outcomes: Vec::new(),
                },
            ],
        }
    }

    #[test]
    fn concept_test_returns_ranked_variants_and_segments() {
        let result = simulate_concept_test(&sample_request()).unwrap();

        assert_eq!(result.total_population, 1_200);
        assert_eq!(result.ranked_variants.len(), 2);
        assert_eq!(result.segment_summaries.len(), 2);
        assert!(!result.sampled_individuals.is_empty());
        assert!(result.recommended_variant_id.is_some());
        assert_eq!(result.calibration_summary.len(), 1);
        assert!(result.touchpoint_calibration_summary.is_empty());
        assert_eq!(result.ranked_variants[0].touchpoint_results.len(), 3);
    }

    #[test]
    fn concept_test_is_deterministic() {
        let request = sample_request();
        let first = simulate_concept_test(&request).unwrap();
        let second = simulate_concept_test(&request).unwrap();

        assert_eq!(
            first.ranked_variants[0].variant_id,
            second.ranked_variants[0].variant_id
        );
        assert_eq!(
            first.ranked_variants[0].overall_score,
            second.ranked_variants[0].overall_score
        );
        assert_eq!(
            first.sampled_individuals[0].top_variant_id,
            second.sampled_individuals[0].top_variant_id
        );
    }

    #[test]
    fn concept_test_rejects_unknown_target_segment() {
        let mut request = sample_request();
        request.variants[0].target_segments = vec!["missing".into()];

        let err = simulate_concept_test(&request).unwrap_err();
        assert!(matches!(err, ConceptTestError::UnknownTargetSegment { .. }));
    }

    #[test]
    fn concept_test_touchpoints_change_scores() {
        let mut without_touchpoints = sample_request();
        without_touchpoints.scenario.touchpoints.clear();
        let without = simulate_concept_test(&without_touchpoints).unwrap();

        let with = simulate_concept_test(&sample_request()).unwrap();

        assert_ne!(
            without.ranked_variants[0].overall_score,
            with.ranked_variants[0].overall_score
        );
        assert!(!with.ranked_variants[0].touchpoint_results.is_empty());
    }

    #[test]
    fn concept_test_rejects_unknown_touchpoint_variant() {
        let mut request = sample_request();
        request.scenario.touchpoints[0].variants = vec!["missing".into()];

        let err = simulate_concept_test(&request).unwrap_err();
        assert!(matches!(
            err,
            ConceptTestError::UnknownTouchpointVariant { .. }
        ));
    }

    #[test]
    fn concept_test_builds_touchpoint_calibration_summary() {
        let mut request = sample_request();
        request.observed_outcomes.push(ConceptObservedOutcome {
            variant_id: "proof_first".into(),
            touchpoint_id: Some("proof".into()),
            segment_id: None,
            source: Some("landing-page-step".into()),
            sample_size: Some(80),
            click_rate: Some(0.48),
            signup_rate: Some(0.24),
            activation_rate: None,
            retention_rate: None,
            referral_rate: None,
        });

        let result = simulate_concept_test(&request).unwrap();

        assert_eq!(result.calibration_summary.len(), 1);
        assert_eq!(result.touchpoint_calibration_summary.len(), 1);
        assert_eq!(
            result.touchpoint_calibration_summary[0]
                .touchpoint_id
                .as_deref(),
            Some("proof")
        );
        assert_eq!(
            result.touchpoint_calibration_summary[0].variant_id,
            "proof_first"
        );
    }

    #[test]
    fn concept_test_rejects_unknown_observed_touchpoint() {
        let mut request = sample_request();
        request.observed_outcomes[0].touchpoint_id = Some("missing".into());

        let err = simulate_concept_test(&request).unwrap_err();
        assert!(matches!(
            err,
            ConceptTestError::UnknownObservedTouchpoint { .. }
        ));
    }

    #[test]
    fn concept_test_matrix_runs_cases_in_order() {
        let request = sample_matrix_request();
        let result = run_concept_test_matrix(&request).unwrap();

        assert_eq!(result.cases.len(), 2);
        assert_eq!(result.cases[0].case_id, "trust");
        assert_eq!(result.cases[1].case_id, "referral");
        assert_eq!(result.variant_rollups.len(), 2);
        assert!(!result.recommendation.is_empty());
        assert_eq!(request.base_request.scenario.id, "wedge");
    }

    #[test]
    fn concept_test_matrix_rejects_empty_cases() {
        let mut request = sample_matrix_request();
        request.cases.clear();

        let err = run_concept_test_matrix(&request).unwrap_err();
        assert!(matches!(err, ConceptTestMatrixError::NoCases));
    }

    #[test]
    fn concept_test_matrix_rejects_duplicate_case_ids() {
        let mut request = sample_matrix_request();
        request.cases[1].id = request.cases[0].id.clone();

        let err = run_concept_test_matrix(&request).unwrap_err();
        assert!(matches!(err, ConceptTestMatrixError::DuplicateCaseId(_)));
    }

    #[test]
    fn concept_test_matrix_surfaces_case_simulation_errors() {
        let mut request = sample_matrix_request();
        request.cases[0].scenario.as_mut().unwrap().time_steps = 0;

        let err = run_concept_test_matrix(&request).unwrap_err();
        assert!(matches!(err, ConceptTestMatrixError::CaseSimulation { .. }));
    }

    #[test]
    fn concept_test_matrix_runs_preset_cases() {
        let mut request = sample_matrix_request();
        request.cases[0].scenario = None;
        request.cases[0].preset = Some(ConceptScenarioPreset::ProcurementSkepticism);
        request.cases[1].scenario = None;
        request.cases[1].preset = Some(ConceptScenarioPreset::GameplayFatigue);

        let result = run_concept_test_matrix(&request).unwrap();

        assert_eq!(result.cases.len(), 2);
        assert_eq!(result.cases[0].scenario_id, "trust");
        assert_eq!(result.cases[1].scenario_id, "referral");
        assert!(!result.cases[0].result.scenario.touchpoints.is_empty());
        assert!(!result.variant_rollups.is_empty());
    }

    #[test]
    fn concept_test_matrix_rejects_case_without_scenario_or_preset() {
        let mut request = sample_matrix_request();
        request.cases[0].scenario = None;

        let err = run_concept_test_matrix(&request).unwrap_err();
        assert!(matches!(
            err,
            ConceptTestMatrixError::MissingCaseScenario(_)
        ));
    }

    #[test]
    fn draft_concept_test_matrix_is_deterministic_and_executable() {
        let draft = ConceptTestMatrixDraftRequest {
            prompt: "Test a founder analytics product. Angle A: proof-led dashboards. Angle B: frictionless setup for busy operators.".into(),
            product_name: Some("Founder Signal Lab".into()),
            case_count: Some(4),
            population: Some(ConceptPopulationConfig {
                target_count: 1_000,
                sample_size: 6,
            }),
            ..ConceptTestMatrixDraftRequest::default()
        };

        let first = draft_concept_test_matrix(&draft).unwrap();
        let second = draft_concept_test_matrix(&draft).unwrap();

        assert_eq!(first.id, second.id);
        assert_eq!(first.base_request.seed_base, second.base_request.seed_base);
        assert_eq!(first.cases.len(), 4);
        assert_eq!(first.base_request.population.target_count, 1_000);
        assert!(first.base_request.variants.len() >= 3);

        let result = run_concept_test_matrix(&first).unwrap();
        assert_eq!(result.cases.len(), 4);
        assert!(!result.variant_rollups.is_empty());
    }

    #[test]
    fn draft_concept_test_matrix_infers_defense_procurement_cases() {
        let draft = ConceptTestMatrixDraftRequest {
            prompt: "Military product for field operators and procurement buyers. Compare mission outcome proof against integration-first deployment.".into(),
            case_count: Some(3),
            ..ConceptTestMatrixDraftRequest::default()
        };

        let matrix = draft_concept_test_matrix(&draft).unwrap();

        assert_eq!(matrix.base_request.population.target_count, 10_000);
        assert!(matrix
            .base_request
            .segments
            .iter()
            .any(|segment| segment.id == "field_operators"));
        assert_eq!(
            matrix.cases[0].preset,
            Some(ConceptScenarioPreset::ProcurementSkepticism)
        );
        assert!(matrix
            .cases
            .iter()
            .any(|case| case.preset == Some(ConceptScenarioPreset::PricingPressure)));
    }

    #[test]
    fn draft_concept_test_matrix_rejects_empty_prompt() {
        let err = draft_concept_test_matrix(&ConceptTestMatrixDraftRequest::default()).unwrap_err();

        assert!(matches!(err, ConceptTestMatrixDraftError::EmptyPrompt));
    }
}
