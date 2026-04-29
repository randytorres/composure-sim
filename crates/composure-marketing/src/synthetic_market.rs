use std::collections::{BTreeMap, BTreeSet};

use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntheticMarketPackage {
    pub market: SyntheticMarketMetadata,
    #[serde(default)]
    pub segments: Vec<SegmentBlueprint>,
    #[serde(default)]
    pub overlap_assumptions: Vec<SegmentOverlapAssumption>,
    #[serde(default)]
    pub frictions: Vec<ProductFrictionPrior>,
    #[serde(default)]
    pub value_drivers: Vec<ValueDriverPrior>,
    #[serde(default)]
    pub channels: Vec<ChannelAssumption>,
    #[serde(default)]
    pub campaign_variants: Vec<CampaignVariantDefinition>,
    #[serde(default)]
    pub scenarios: Vec<SyntheticScenarioDefinition>,
    #[serde(default)]
    pub observed_outcomes: Vec<SyntheticObservedOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntheticMarketMetadata {
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default)]
    pub region: Option<String>,
    #[serde(default)]
    pub pricing_reference: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentBlueprint {
    pub segment_id: String,
    pub name: String,
    #[serde(default)]
    pub priority: Option<String>,
    pub share_prior: f64,
    #[serde(default)]
    pub adjacent_to: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub demographic_shape: BTreeMap<String, String>,
    #[serde(default)]
    pub jobs: Vec<String>,
    #[serde(default)]
    pub traits: BTreeMap<String, TraitDistribution>,
    #[serde(default)]
    pub preferred_channels: Vec<String>,
    #[serde(default)]
    pub objection_clusters: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraitDistribution {
    #[serde(default = "default_distribution_name")]
    pub distribution: String,
    #[serde(default)]
    pub alpha: f64,
    #[serde(default)]
    pub beta: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentOverlapAssumption {
    pub from_segment: String,
    pub to_segment: String,
    pub overlap_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactProfile {
    #[serde(default)]
    pub signup_penalty: f64,
    #[serde(default)]
    pub activation_penalty: f64,
    #[serde(default)]
    pub retention_penalty: f64,
}

impl ImpactProfile {
    fn average_penalty(&self) -> f64 {
        (self.signup_penalty + self.activation_penalty + self.retention_penalty) / 3.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductFrictionPrior {
    pub friction_id: String,
    pub label: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub default_impact: Option<ImpactProfile>,
    #[serde(default)]
    pub segment_modifiers: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueDriverPrior {
    pub driver_id: String,
    pub label: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub segment_lift: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelAssumption {
    pub channel_id: String,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub fit_notes: Vec<String>,
    #[serde(default)]
    pub reach_priority: Option<String>,
    pub trust_base: f64,
    pub creator_lift: f64,
    pub friction_tolerance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignVariantDefinition {
    pub variant_id: String,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub best_for_segments: Vec<String>,
    #[serde(default)]
    pub channel_fit: Vec<String>,
    #[serde(default)]
    pub supports_value_drivers: Vec<String>,
    #[serde(default)]
    pub exposed_frictions: Vec<String>,
    #[serde(default)]
    pub core_strengths: Vec<String>,
    #[serde(default)]
    pub core_risks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntheticScenarioDefinition {
    pub scenario_id: String,
    pub goal: String,
    pub decision: String,
    #[serde(default)]
    pub primary_segments: Vec<String>,
    #[serde(default)]
    pub secondary_segments: Vec<String>,
    #[serde(default)]
    pub campaign_variants: Vec<String>,
    #[serde(default)]
    pub channels: Vec<String>,
    #[serde(default)]
    pub key_questions: Vec<String>,
    #[serde(default)]
    pub success_metrics: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntheticObservedOutcome {
    pub experiment_id: String,
    pub scenario_id: String,
    #[serde(alias = "approach_id")]
    pub variant_id: String,
    #[serde(default)]
    pub segment_id: Option<String>,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub creative_id: Option<String>,
    #[serde(default)]
    pub hook_id: Option<String>,
    #[serde(default)]
    pub landing_variant: Option<String>,
    #[serde(default)]
    pub sample_size: Option<u32>,
    #[serde(default)]
    pub metrics: SyntheticObservedMetrics,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyntheticObservedMetrics {
    #[serde(default)]
    pub click_through_rate: Option<f64>,
    #[serde(default)]
    pub signup_rate: Option<f64>,
    #[serde(default)]
    pub activation_rate: Option<f64>,
    #[serde(default)]
    pub week_2_retention: Option<f64>,
    #[serde(default)]
    pub paid_conversion_rate: Option<f64>,
    #[serde(default)]
    pub share_rate: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntheticMarketSimulationResult {
    pub market_name: String,
    pub scenario_id: String,
    pub scenario_goal: String,
    pub scenario_decision: String,
    pub total_buyers_simulated: usize,
    pub market_funnel: AggregateFunnelMetrics,
    pub observed_data_summary: ObservedDataSummary,
    pub calibration_summary: Vec<SyntheticCalibrationSummary>,
    pub business_readiness: BusinessReadinessSummary,
    pub recommended_control: String,
    #[serde(default)]
    pub recommended_challenger: Option<String>,
    pub ranked_variants: Vec<VariantScenarioScore>,
    pub segment_summaries: Vec<SegmentScenarioSummary>,
    #[serde(default)]
    pub sampled_buyers: Vec<SyntheticBuyerSample>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantScenarioScore {
    pub variant_id: String,
    pub role: Option<String>,
    pub overall_score: u32,
    pub weighted_segment_share: f64,
    pub funnel: AggregateFunnelMetrics,
    pub strongest_segments: Vec<String>,
    pub weakest_segments: Vec<String>,
    pub risk_flags: Vec<String>,
    pub segment_scores: Vec<VariantSegmentScore>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantSegmentScore {
    pub segment_id: String,
    pub segment_name: String,
    pub effective_weight: f64,
    pub score: u32,
    pub funnel: AggregateFunnelMetrics,
    pub affinity_score: u32,
    pub channel_fit_score: u32,
    pub trust_score: u32,
    pub driver_score: u32,
    pub friction_penalty: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentScenarioSummary {
    pub segment_id: String,
    pub segment_name: String,
    pub effective_weight: f64,
    pub buyers_simulated: usize,
    pub best_variant_funnel: AggregateFunnelMetrics,
    pub best_variant_id: String,
    pub best_score: u32,
    #[serde(default)]
    pub runner_up_variant_id: Option<String>,
    #[serde(default)]
    pub runner_up_score: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntheticBuyerSample {
    pub buyer_id: String,
    pub segment_id: String,
    pub strongest_variant_id: String,
    pub strongest_variant_score: u32,
    #[serde(default)]
    pub runner_up_variant_id: Option<String>,
    #[serde(default)]
    pub runner_up_score: Option<u32>,
    pub proof_hunger: u32,
    pub manual_logging_tolerance: u32,
    pub privacy_sensitivity: u32,
    pub wearable_ownership: u32,
    pub subscription_willingness: u32,
    pub click_probability: u32,
    pub signup_probability: u32,
    pub activation_probability: u32,
    pub retention_probability: u32,
    pub paid_conversion_probability: u32,
    pub clicked: bool,
    pub signed_up: bool,
    pub activated: bool,
    pub retained: bool,
    pub converted_paid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntheticCalibrationSummary {
    pub variant_id: String,
    pub observed_records: usize,
    pub usable_observed_records: usize,
    pub placeholder_records: usize,
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
    pub paid_conversion_gap: Option<f64>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservedDataSummary {
    pub records: usize,
    pub usable_records: usize,
    pub placeholder_records: usize,
    #[serde(default)]
    pub total_usable_sample_size: Option<u32>,
    #[serde(default)]
    pub organic_sources: Vec<String>,
    #[serde(default)]
    pub paid_sources: Vec<String>,
    pub acquisition_motion: String,
    pub data_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessReadinessSummary {
    pub acquisition_motion: String,
    pub observed_data_status: String,
    pub organic_readiness_score: u32,
    pub paid_readiness_score: u32,
    pub subscription_readiness_score: u32,
    pub current_focus: String,
    #[serde(default)]
    pub gating_factors: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AggregateFunnelMetrics {
    pub buyers: usize,
    pub clicks: usize,
    pub signups: usize,
    pub activations: usize,
    pub retained: usize,
    pub paid_conversions: usize,
    pub click_rate: f64,
    pub signup_rate: f64,
    pub activation_rate: f64,
    pub retention_rate: f64,
    pub paid_conversion_rate: f64,
}

#[derive(Debug, Error, PartialEq, Clone)]
pub enum SyntheticMarketValidationError {
    #[error("market name cannot be empty")]
    EmptyMarketName,
    #[error("at least one segment is required")]
    MissingSegments,
    #[error("segment IDs must be unique")]
    DuplicateSegmentId,
    #[error("segment share priors must be within 0.0..=1.0")]
    InvalidSegmentShare,
    #[error("segment share priors must sum close to 1.0")]
    InvalidSegmentShareTotal,
    #[error("segment adjacency references an unknown segment")]
    UnknownAdjacentSegment,
    #[error("segment overlap references an unknown segment")]
    UnknownOverlapSegment,
    #[error("segment overlap score must be within 0.0..=1.0")]
    InvalidOverlapScore,
    #[error("friction IDs must be unique")]
    DuplicateFrictionId,
    #[error("friction modifiers reference an unknown segment")]
    UnknownFrictionSegment,
    #[error("friction modifiers must be positive")]
    InvalidFrictionModifier,
    #[error("value driver IDs must be unique")]
    DuplicateValueDriverId,
    #[error("value driver lift references an unknown segment")]
    UnknownValueDriverSegment,
    #[error("value driver lift must be positive")]
    InvalidValueDriverLift,
    #[error("channel IDs must be unique")]
    DuplicateChannelId,
    #[error("channel assumptions must use values within 0.0..=1.0")]
    InvalidChannelValue,
    #[error("campaign variant IDs must be unique")]
    DuplicateVariantId,
    #[error("campaign variants reference an unknown segment")]
    UnknownVariantSegment,
    #[error("campaign variants reference an unknown channel")]
    UnknownVariantChannel,
    #[error("campaign variants reference an unknown value driver")]
    UnknownVariantValueDriver,
    #[error("campaign variants reference an unknown friction")]
    UnknownVariantFriction,
    #[error("scenario IDs must be unique")]
    DuplicateScenarioId,
    #[error("scenarios must include at least one primary segment")]
    MissingScenarioPrimarySegments,
    #[error("scenarios reference an unknown segment")]
    UnknownScenarioSegment,
    #[error("scenarios reference an unknown campaign variant")]
    UnknownScenarioVariant,
    #[error("scenarios reference an unknown channel")]
    UnknownScenarioChannel,
    #[error("scenarios must include at least one success metric")]
    MissingScenarioMetrics,
    #[error("observed outcomes reference an unknown scenario")]
    UnknownObservedScenario,
    #[error("observed outcomes reference an unknown campaign variant")]
    UnknownObservedVariant,
    #[error("observed outcomes reference an unknown segment")]
    UnknownObservedSegment,
    #[error("observed outcome rates must be within 0.0..=1.0")]
    InvalidObservedRate,
}

#[derive(Debug, Error, PartialEq)]
pub enum SyntheticMarketSimulationError {
    #[error("synthetic market package validation failed: {0}")]
    Validation(#[from] SyntheticMarketValidationError),
    #[error("scenario {0} was not found in the package")]
    ScenarioNotFound(String),
}

impl SyntheticMarketPackage {
    pub fn validate(&self) -> Result<(), SyntheticMarketValidationError> {
        if self.market.name.trim().is_empty() {
            return Err(SyntheticMarketValidationError::EmptyMarketName);
        }
        if self.segments.is_empty() {
            return Err(SyntheticMarketValidationError::MissingSegments);
        }

        let mut segment_ids = BTreeSet::new();
        let mut share_total = 0.0;
        for segment in &self.segments {
            if !segment_ids.insert(segment.segment_id.as_str()) {
                return Err(SyntheticMarketValidationError::DuplicateSegmentId);
            }
            if !(0.0..=1.0).contains(&segment.share_prior) {
                return Err(SyntheticMarketValidationError::InvalidSegmentShare);
            }
            share_total += segment.share_prior;
        }
        if !(0.95..=1.05).contains(&share_total) {
            return Err(SyntheticMarketValidationError::InvalidSegmentShareTotal);
        }

        for segment in &self.segments {
            for adjacent in &segment.adjacent_to {
                if !segment_ids.contains(adjacent.as_str()) {
                    return Err(SyntheticMarketValidationError::UnknownAdjacentSegment);
                }
            }
        }

        for overlap in &self.overlap_assumptions {
            if !segment_ids.contains(overlap.from_segment.as_str())
                || !segment_ids.contains(overlap.to_segment.as_str())
            {
                return Err(SyntheticMarketValidationError::UnknownOverlapSegment);
            }
            if !(0.0..=1.0).contains(&overlap.overlap_score) {
                return Err(SyntheticMarketValidationError::InvalidOverlapScore);
            }
        }

        let friction_ids = validate_segment_maps(
            &self.frictions,
            |item| &item.friction_id,
            |item| &item.segment_modifiers,
            &segment_ids,
            SyntheticMarketValidationError::DuplicateFrictionId,
            SyntheticMarketValidationError::UnknownFrictionSegment,
            SyntheticMarketValidationError::InvalidFrictionModifier,
        )?;

        let value_driver_ids = validate_segment_maps(
            &self.value_drivers,
            |item| &item.driver_id,
            |item| &item.segment_lift,
            &segment_ids,
            SyntheticMarketValidationError::DuplicateValueDriverId,
            SyntheticMarketValidationError::UnknownValueDriverSegment,
            SyntheticMarketValidationError::InvalidValueDriverLift,
        )?;

        let mut channel_ids = BTreeSet::new();
        for channel in &self.channels {
            if !channel_ids.insert(channel.channel_id.as_str()) {
                return Err(SyntheticMarketValidationError::DuplicateChannelId);
            }
            if !(0.0..=1.0).contains(&channel.trust_base)
                || !(0.0..=1.0).contains(&channel.creator_lift)
                || !(0.0..=1.0).contains(&channel.friction_tolerance)
            {
                return Err(SyntheticMarketValidationError::InvalidChannelValue);
            }
        }

        let mut variant_ids = BTreeSet::new();
        for variant in &self.campaign_variants {
            if !variant_ids.insert(variant.variant_id.as_str()) {
                return Err(SyntheticMarketValidationError::DuplicateVariantId);
            }
            for segment in &variant.best_for_segments {
                if !segment_ids.contains(segment.as_str()) {
                    return Err(SyntheticMarketValidationError::UnknownVariantSegment);
                }
            }
            for channel in &variant.channel_fit {
                if !channel_ids.contains(channel.as_str()) {
                    return Err(SyntheticMarketValidationError::UnknownVariantChannel);
                }
            }
            for driver in &variant.supports_value_drivers {
                if !value_driver_ids.contains(driver.as_str()) {
                    return Err(SyntheticMarketValidationError::UnknownVariantValueDriver);
                }
            }
            for friction in &variant.exposed_frictions {
                if !friction_ids.contains(friction.as_str()) {
                    return Err(SyntheticMarketValidationError::UnknownVariantFriction);
                }
            }
        }

        let mut scenario_ids = BTreeSet::new();
        for scenario in &self.scenarios {
            if !scenario_ids.insert(scenario.scenario_id.as_str()) {
                return Err(SyntheticMarketValidationError::DuplicateScenarioId);
            }
            if scenario.primary_segments.is_empty() {
                return Err(SyntheticMarketValidationError::MissingScenarioPrimarySegments);
            }
            if scenario.success_metrics.is_empty() {
                return Err(SyntheticMarketValidationError::MissingScenarioMetrics);
            }
            for segment in scenario
                .primary_segments
                .iter()
                .chain(scenario.secondary_segments.iter())
            {
                if !segment_ids.contains(segment.as_str()) {
                    return Err(SyntheticMarketValidationError::UnknownScenarioSegment);
                }
            }
            for variant in &scenario.campaign_variants {
                if !variant_ids.contains(variant.as_str()) {
                    return Err(SyntheticMarketValidationError::UnknownScenarioVariant);
                }
            }
            for channel in &scenario.channels {
                if !channel_ids.contains(channel.as_str()) {
                    return Err(SyntheticMarketValidationError::UnknownScenarioChannel);
                }
            }
        }

        for outcome in &self.observed_outcomes {
            if !scenario_ids.contains(outcome.scenario_id.as_str()) {
                return Err(SyntheticMarketValidationError::UnknownObservedScenario);
            }
            if !variant_ids.contains(outcome.variant_id.as_str()) {
                return Err(SyntheticMarketValidationError::UnknownObservedVariant);
            }
            if let Some(segment_id) = &outcome.segment_id {
                if !segment_ids.contains(segment_id.as_str()) {
                    return Err(SyntheticMarketValidationError::UnknownObservedSegment);
                }
            }
            for rate in [
                outcome.metrics.click_through_rate,
                outcome.metrics.signup_rate,
                outcome.metrics.activation_rate,
                outcome.metrics.week_2_retention,
                outcome.metrics.paid_conversion_rate,
                outcome.metrics.share_rate,
            ]
            .into_iter()
            .flatten()
            {
                if !(0.0..=1.0).contains(&rate) {
                    return Err(SyntheticMarketValidationError::InvalidObservedRate);
                }
            }
        }

        Ok(())
    }
}

pub fn simulate_synthetic_market(
    package: &SyntheticMarketPackage,
    scenario_id: &str,
) -> Result<SyntheticMarketSimulationResult, SyntheticMarketSimulationError> {
    package.validate()?;

    let scenario = package
        .scenarios
        .iter()
        .find(|item| item.scenario_id == scenario_id)
        .ok_or_else(|| SyntheticMarketSimulationError::ScenarioNotFound(scenario_id.into()))?;

    let segments_by_id = package
        .segments
        .iter()
        .map(|segment| (segment.segment_id.as_str(), segment))
        .collect::<BTreeMap<_, _>>();
    let channels_by_id = package
        .channels
        .iter()
        .map(|channel| (channel.channel_id.as_str(), channel))
        .collect::<BTreeMap<_, _>>();
    let frictions_by_id = package
        .frictions
        .iter()
        .map(|friction| (friction.friction_id.as_str(), friction))
        .collect::<BTreeMap<_, _>>();
    let value_drivers_by_id = package
        .value_drivers
        .iter()
        .map(|driver| (driver.driver_id.as_str(), driver))
        .collect::<BTreeMap<_, _>>();
    let variants_by_id = package
        .campaign_variants
        .iter()
        .map(|variant| (variant.variant_id.as_str(), variant))
        .collect::<BTreeMap<_, _>>();

    let segment_weights = scenario
        .primary_segments
        .iter()
        .map(|segment_id| (segment_id.as_str(), 1.0))
        .chain(
            scenario
                .secondary_segments
                .iter()
                .map(|segment_id| (segment_id.as_str(), 0.65)),
        )
        .collect::<BTreeMap<_, _>>();

    let normalized_weights = normalized_segment_weights(&segment_weights, &segments_by_id);
    let total_buyers_simulated = 2048usize;
    let scenario_variants = scenario
        .campaign_variants
        .iter()
        .filter_map(|variant_id| variants_by_id.get(variant_id.as_str()).copied())
        .collect::<Vec<_>>();

    let mut buyer_rng =
        StdRng::seed_from_u64(simulation_seed(&package.market.name, &scenario.scenario_id));
    let mut segment_buyer_counts = BTreeMap::new();
    let mut sampled_buyers = Vec::new();
    let mut per_variant_segment_scores: BTreeMap<String, Vec<VariantSegmentScore>> =
        BTreeMap::new();

    for (segment_id, normalized_weight) in &normalized_weights {
        let segment = segments_by_id[segment_id];
        let buyers_for_segment =
            ((total_buyers_simulated as f64 * normalized_weight).round() as usize).max(96);
        segment_buyer_counts.insert(segment.segment_id.clone(), buyers_for_segment);

        let mut per_variant_accumulators: BTreeMap<String, FunnelAccumulator> = BTreeMap::new();
        for buyer_index in 0..buyers_for_segment {
            let buyer = sample_buyer(segment, &mut buyer_rng);
            let mut ranked_for_buyer = scenario_variants
                .iter()
                .map(|variant| {
                    let variant_score = score_buyer_against_variant(
                        &buyer,
                        segment,
                        variant,
                        scenario,
                        &mut buyer_rng,
                        &channels_by_id,
                        &frictions_by_id,
                        &value_drivers_by_id,
                    );
                    let accumulator = per_variant_accumulators
                        .entry(variant.variant_id.clone())
                        .or_default();
                    accumulator.buyers += 1;
                    accumulator.score_sum += variant_score.score;
                    accumulator.clicks += usize::from(variant_score.funnel.clicked);
                    accumulator.signups += usize::from(variant_score.funnel.signed_up);
                    accumulator.activations += usize::from(variant_score.funnel.activated);
                    accumulator.retained += usize::from(variant_score.funnel.retained);
                    accumulator.paid_conversions +=
                        usize::from(variant_score.funnel.converted_paid);
                    (
                        variant.variant_id.clone(),
                        as_percent(variant_score.score),
                        variant_score.funnel,
                    )
                })
                .collect::<Vec<_>>();
            ranked_for_buyer.sort_by(|a, b| b.1.cmp(&a.1));

            if sampled_buyers.len() < 18
                && buyer_index % (buyers_for_segment.max(1) / 3).max(1) == 0
            {
                sampled_buyers.push(SyntheticBuyerSample {
                    buyer_id: format!("{}-buyer-{}", segment.segment_id, buyer_index + 1),
                    segment_id: segment.segment_id.clone(),
                    strongest_variant_id: ranked_for_buyer
                        .first()
                        .map(|item| item.0.clone())
                        .unwrap_or_default(),
                    strongest_variant_score: ranked_for_buyer
                        .first()
                        .map(|item| item.1)
                        .unwrap_or_default(),
                    runner_up_variant_id: ranked_for_buyer.get(1).map(|item| item.0.clone()),
                    runner_up_score: ranked_for_buyer.get(1).map(|item| item.1),
                    proof_hunger: as_percent(buyer.proof_hunger),
                    manual_logging_tolerance: as_percent(buyer.manual_logging_tolerance),
                    privacy_sensitivity: as_percent(buyer.privacy_sensitivity),
                    wearable_ownership: as_percent(buyer.wearable_ownership),
                    subscription_willingness: as_percent(buyer.subscription_willingness),
                    click_probability: ranked_for_buyer
                        .first()
                        .map(|item| as_percent(item.2.click_probability))
                        .unwrap_or_default(),
                    signup_probability: ranked_for_buyer
                        .first()
                        .map(|item| as_percent(item.2.signup_probability))
                        .unwrap_or_default(),
                    activation_probability: ranked_for_buyer
                        .first()
                        .map(|item| as_percent(item.2.activation_probability))
                        .unwrap_or_default(),
                    retention_probability: ranked_for_buyer
                        .first()
                        .map(|item| as_percent(item.2.retention_probability))
                        .unwrap_or_default(),
                    paid_conversion_probability: ranked_for_buyer
                        .first()
                        .map(|item| as_percent(item.2.paid_conversion_probability))
                        .unwrap_or_default(),
                    clicked: ranked_for_buyer
                        .first()
                        .map(|item| item.2.clicked)
                        .unwrap_or(false),
                    signed_up: ranked_for_buyer
                        .first()
                        .map(|item| item.2.signed_up)
                        .unwrap_or(false),
                    activated: ranked_for_buyer
                        .first()
                        .map(|item| item.2.activated)
                        .unwrap_or(false),
                    retained: ranked_for_buyer
                        .first()
                        .map(|item| item.2.retained)
                        .unwrap_or(false),
                    converted_paid: ranked_for_buyer
                        .first()
                        .map(|item| item.2.converted_paid)
                        .unwrap_or(false),
                });
            }
        }

        for variant in &scenario_variants {
            let accumulator = per_variant_accumulators
                .get(&variant.variant_id)
                .cloned()
                .unwrap_or_default();
            let avg_score = if accumulator.buyers == 0 {
                0.0
            } else {
                accumulator.score_sum / accumulator.buyers as f64
            };
            let affinity = affinity_score(segment, variant);
            let channel_fit = channel_overlap_score(&scenario.channels, &variant.channel_fit);
            let trust = trust_score(&scenario.channels, &variant.channel_fit, &channels_by_id);
            let driver = value_driver_score_for_segment(
                segment,
                &variant.supports_value_drivers,
                &value_drivers_by_id,
            );
            let penalty =
                friction_penalty_for_segment(segment, &variant.exposed_frictions, &frictions_by_id);
            per_variant_segment_scores
                .entry(variant.variant_id.clone())
                .or_default()
                .push(VariantSegmentScore {
                    segment_id: segment.segment_id.clone(),
                    segment_name: segment.name.clone(),
                    effective_weight: normalized_weight * buyers_for_segment as f64
                        / total_buyers_simulated as f64,
                    score: as_percent(avg_score),
                    funnel: aggregate_funnel_metrics(&accumulator),
                    affinity_score: as_percent(affinity),
                    channel_fit_score: as_percent(channel_fit),
                    trust_score: as_percent(trust),
                    driver_score: as_percent(driver),
                    friction_penalty: as_percent(penalty),
                });
        }
    }

    let mut ranked_variants = scenario_variants
        .iter()
        .map(|variant| {
            let mut segment_scores = per_variant_segment_scores
                .remove(&variant.variant_id)
                .unwrap_or_default();
            segment_scores.sort_by(|a, b| b.score.cmp(&a.score));
            let strongest_segments = segment_scores
                .iter()
                .take(2)
                .map(|item| item.segment_id.clone())
                .collect::<Vec<_>>();
            let weakest_segments = segment_scores
                .iter()
                .rev()
                .take(2)
                .map(|item| item.segment_id.clone())
                .collect::<Vec<_>>();
            let weighted_total = segment_scores
                .iter()
                .map(|item| item.effective_weight * item.score as f64)
                .sum::<f64>();
            let total_weight = segment_scores
                .iter()
                .map(|item| item.effective_weight)
                .sum::<f64>()
                .max(f64::EPSILON);
            VariantScenarioScore {
                variant_id: variant.variant_id.clone(),
                role: variant.role.clone(),
                overall_score: (weighted_total / total_weight).round() as u32,
                weighted_segment_share: total_weight,
                funnel: aggregate_weighted_funnel(&segment_scores),
                strongest_segments,
                weakest_segments,
                risk_flags: build_risk_flags(variant, &scenario.channels),
                segment_scores,
            }
        })
        .collect::<Vec<_>>();

    ranked_variants.sort_by(|a, b| b.overall_score.cmp(&a.overall_score));

    let recommended_control = ranked_variants
        .first()
        .map(|item| item.variant_id.clone())
        .unwrap_or_default();
    let recommended_challenger = ranked_variants.get(1).map(|item| item.variant_id.clone());
    let observed_data_summary = build_observed_data_summary(package, scenario);
    let calibration_summary = build_calibration_summary(package, scenario, &ranked_variants);
    let business_readiness = build_business_readiness(
        &observed_data_summary,
        &ranked_variants,
        &recommended_control,
    );

    let mut segment_summaries = Vec::new();
    for (segment_id, multiplier) in &segment_weights {
        let segment = segments_by_id[segment_id];
        let mut per_variant = ranked_variants
            .iter()
            .filter_map(|variant| {
                variant
                    .segment_scores
                    .iter()
                    .find(|score| score.segment_id == segment.segment_id)
                    .map(|score| (variant.variant_id.clone(), score.score))
            })
            .collect::<Vec<_>>();
        per_variant.sort_by(|a, b| b.1.cmp(&a.1));
        let best = per_variant.first().cloned().unwrap_or_default();
        let runner_up = per_variant.get(1).cloned();
        segment_summaries.push(SegmentScenarioSummary {
            segment_id: segment.segment_id.clone(),
            segment_name: segment.name.clone(),
            effective_weight: segment.share_prior * *multiplier,
            buyers_simulated: *segment_buyer_counts.get(&segment.segment_id).unwrap_or(&0),
            best_variant_funnel: ranked_variants
                .iter()
                .find(|variant| variant.variant_id == best.0)
                .and_then(|variant| {
                    variant
                        .segment_scores
                        .iter()
                        .find(|score| score.segment_id == segment.segment_id)
                        .map(|score| score.funnel.clone())
                })
                .unwrap_or_default(),
            best_variant_id: best.0,
            best_score: best.1,
            runner_up_variant_id: runner_up.as_ref().map(|item| item.0.clone()),
            runner_up_score: runner_up.map(|item| item.1),
        });
    }
    segment_summaries.sort_by(|a, b| b.best_score.cmp(&a.best_score));

    Ok(SyntheticMarketSimulationResult {
        market_name: package.market.name.clone(),
        scenario_id: scenario.scenario_id.clone(),
        scenario_goal: scenario.goal.clone(),
        scenario_decision: scenario.decision.clone(),
        total_buyers_simulated,
        market_funnel: aggregate_market_funnel(&ranked_variants, &recommended_control),
        observed_data_summary: observed_data_summary.clone(),
        calibration_summary,
        business_readiness: business_readiness.clone(),
        recommended_control,
        recommended_challenger,
        ranked_variants,
        segment_summaries: segment_summaries.clone(),
        sampled_buyers,
        notes: build_simulation_notes(
            scenario,
            &segment_summaries,
            &observed_data_summary,
            &business_readiness,
        ),
    })
}

fn validate_segment_maps<T>(
    items: &[T],
    id_fn: impl Fn(&T) -> &str,
    segment_map_fn: impl Fn(&T) -> &BTreeMap<String, f64>,
    valid_segments: &BTreeSet<&str>,
    duplicate_err: SyntheticMarketValidationError,
    unknown_segment_err: SyntheticMarketValidationError,
    invalid_value_err: SyntheticMarketValidationError,
) -> Result<BTreeSet<String>, SyntheticMarketValidationError> {
    let mut ids = BTreeSet::new();
    for item in items {
        let id = id_fn(item);
        if !ids.insert(id.to_string()) {
            return Err(duplicate_err.clone());
        }
        for (key, value) in segment_map_fn(item) {
            if !valid_segments.contains(key.as_str()) {
                return Err(unknown_segment_err.clone());
            }
            if *value <= 0.0 {
                return Err(invalid_value_err.clone());
            }
        }
    }
    Ok(ids)
}

#[derive(Debug, Clone)]
struct BuyerProfile {
    proof_hunger: f64,
    manual_logging_tolerance: f64,
    privacy_sensitivity: f64,
    wearable_ownership: f64,
    peptide_openness: f64,
    scientific_rigor_threshold: f64,
    creator_trust: f64,
    weekly_review_desire: f64,
    subscription_willingness: f64,
    passive_data_expectation: f64,
    trust_barrier: f64,
    dashboard_tolerance: f64,
}

#[derive(Debug, Clone)]
struct BuyerVariantScore {
    score: f64,
    funnel: BuyerFunnelOutcome,
}

#[derive(Debug, Clone, Default)]
struct BuyerFunnelOutcome {
    click_probability: f64,
    signup_probability: f64,
    activation_probability: f64,
    retention_probability: f64,
    paid_conversion_probability: f64,
    clicked: bool,
    signed_up: bool,
    activated: bool,
    retained: bool,
    converted_paid: bool,
}

#[derive(Debug, Clone, Default)]
struct FunnelAccumulator {
    buyers: usize,
    score_sum: f64,
    clicks: usize,
    signups: usize,
    activations: usize,
    retained: usize,
    paid_conversions: usize,
}

fn normalized_segment_weights<'a>(
    segment_weights: &BTreeMap<&'a str, f64>,
    segments_by_id: &BTreeMap<&'a str, &'a SegmentBlueprint>,
) -> BTreeMap<&'a str, f64> {
    let total = segment_weights
        .iter()
        .map(|(segment_id, multiplier)| segments_by_id[segment_id].share_prior * *multiplier)
        .sum::<f64>()
        .max(f64::EPSILON);

    segment_weights
        .iter()
        .map(|(segment_id, multiplier)| {
            (
                *segment_id,
                (segments_by_id[segment_id].share_prior * *multiplier) / total,
            )
        })
        .collect()
}

fn simulation_seed(market_name: &str, scenario_id: &str) -> u64 {
    let mut hash = 1469598103934665603u64;
    for byte in market_name.bytes().chain(scenario_id.bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}

fn sample_buyer(segment: &SegmentBlueprint, rng: &mut StdRng) -> BuyerProfile {
    BuyerProfile {
        proof_hunger: sample_named_trait(segment, "proof_hunger", 0.62, rng),
        manual_logging_tolerance: sample_named_trait(
            segment,
            "manual_logging_tolerance",
            0.45,
            rng,
        ),
        privacy_sensitivity: sample_named_trait(segment, "privacy_sensitivity", 0.42, rng),
        wearable_ownership: sample_named_trait(segment, "wearable_ownership", 0.5, rng),
        peptide_openness: sample_named_trait(segment, "peptide_openness", 0.4, rng),
        scientific_rigor_threshold: sample_named_trait(
            segment,
            "scientific_rigor_threshold",
            0.55,
            rng,
        ),
        creator_trust: sample_named_trait(segment, "creator_trust", 0.5, rng),
        weekly_review_desire: sample_named_trait(segment, "weekly_review_desire", 0.58, rng),
        subscription_willingness: sample_named_trait(
            segment,
            "subscription_willingness",
            0.52,
            rng,
        ),
        passive_data_expectation: sample_named_trait(segment, "passive_data_expectation", 0.5, rng),
        trust_barrier: sample_named_trait(segment, "trust_barrier", 0.5, rng),
        dashboard_tolerance: sample_named_trait(segment, "dashboard_tolerance", 0.45, rng),
    }
}

fn sample_named_trait(
    segment: &SegmentBlueprint,
    trait_name: &str,
    fallback: f64,
    rng: &mut StdRng,
) -> f64 {
    let Some(distribution) = segment.traits.get(trait_name) else {
        return clamp01(fallback + rng.gen_range(-0.08..0.08));
    };
    approximate_beta_sample(distribution, rng)
}

fn approximate_beta_sample(distribution: &TraitDistribution, rng: &mut StdRng) -> f64 {
    let alpha = distribution.alpha.max(1.0);
    let beta = distribution.beta.max(1.0);
    let mean = alpha / (alpha + beta);
    let concentration = (alpha + beta).sqrt();
    let jitter = (0.55 / concentration).clamp(0.03, 0.18);
    clamp01(mean + rng.gen_range(-jitter..jitter))
}

fn score_buyer_against_variant(
    buyer: &BuyerProfile,
    segment: &SegmentBlueprint,
    variant: &CampaignVariantDefinition,
    scenario: &SyntheticScenarioDefinition,
    rng: &mut StdRng,
    channels_by_id: &BTreeMap<&str, &ChannelAssumption>,
    frictions_by_id: &BTreeMap<&str, &ProductFrictionPrior>,
    value_drivers_by_id: &BTreeMap<&str, &ValueDriverPrior>,
) -> BuyerVariantScore {
    let affinity = affinity_score(segment, variant);
    let channel_fit = personalized_channel_fit(segment, buyer, scenario, variant);
    let trust = buyer_trust_score(buyer, segment, scenario, variant, channels_by_id);
    let driver = buyer_value_driver_score(
        buyer,
        segment,
        &variant.supports_value_drivers,
        value_drivers_by_id,
    );
    let penalty =
        buyer_friction_penalty(buyer, segment, &variant.exposed_frictions, frictions_by_id);
    let role_bonus = role_bonus(variant.role.as_deref());
    let score = clamp01(
        0.14 + (affinity * 0.24) + (channel_fit * 0.16) + (trust * 0.15) + (driver * 0.29)
            - (penalty * 0.24)
            + (buyer.subscription_willingness * 0.08)
            + variant_specific_adjustment(buyer, variant)
            + role_bonus,
    );
    let funnel = simulate_buyer_funnel(buyer, score, channel_fit, trust, driver, penalty, rng);
    BuyerVariantScore { score, funnel }
}

fn affinity_score(segment: &SegmentBlueprint, variant: &CampaignVariantDefinition) -> f64 {
    if variant
        .best_for_segments
        .iter()
        .any(|id| id == &segment.segment_id)
    {
        1.0
    } else if segment
        .adjacent_to
        .iter()
        .any(|adjacent| variant.best_for_segments.contains(adjacent))
    {
        0.62
    } else {
        0.38
    }
}

fn channel_overlap_score(scenario_channels: &[String], variant_channels: &[String]) -> f64 {
    if scenario_channels.is_empty() || variant_channels.is_empty() {
        return 0.4;
    }
    let overlap = scenario_channels
        .iter()
        .filter(|channel| variant_channels.contains(channel))
        .count();
    overlap as f64 / scenario_channels.len() as f64
}

fn personalized_channel_fit(
    segment: &SegmentBlueprint,
    buyer: &BuyerProfile,
    scenario: &SyntheticScenarioDefinition,
    variant: &CampaignVariantDefinition,
) -> f64 {
    let overlap = channel_overlap_score(&scenario.channels, &variant.channel_fit);
    let preferred_overlap = if segment.preferred_channels.is_empty() {
        0.5
    } else {
        channel_overlap_score(&segment.preferred_channels, &variant.channel_fit)
    };
    let creator_multiplier = if variant
        .channel_fit
        .iter()
        .any(|channel| channel == "tiktok" || channel == "instagram" || channel == "youtube")
    {
        buyer.creator_trust
    } else {
        0.45
    };
    clamp01((overlap * 0.45) + (preferred_overlap * 0.35) + (creator_multiplier * 0.20))
}

fn trust_score(
    scenario_channels: &[String],
    variant_channels: &[String],
    channels_by_id: &BTreeMap<&str, &ChannelAssumption>,
) -> f64 {
    let relevant = scenario_channels
        .iter()
        .filter(|channel| variant_channels.contains(channel))
        .filter_map(|channel| channels_by_id.get(channel.as_str()).copied())
        .collect::<Vec<_>>();
    if relevant.is_empty() {
        return 0.5;
    }
    relevant
        .iter()
        .map(|channel| (channel.trust_base * 0.75) + (channel.creator_lift * 0.25))
        .sum::<f64>()
        / relevant.len() as f64
}

fn buyer_trust_score(
    buyer: &BuyerProfile,
    segment: &SegmentBlueprint,
    scenario: &SyntheticScenarioDefinition,
    variant: &CampaignVariantDefinition,
    channels_by_id: &BTreeMap<&str, &ChannelAssumption>,
) -> f64 {
    let base = trust_score(&scenario.channels, &variant.channel_fit, channels_by_id);
    let privacy_bonus = if variant
        .supports_value_drivers
        .iter()
        .any(|driver| driver == "private_exportable_system")
    {
        buyer.privacy_sensitivity * 0.18
    } else {
        0.0
    };
    let rigor_penalty = if variant
        .core_risks
        .iter()
        .any(|risk| risk.to_ascii_lowercase().contains("generic"))
    {
        buyer.scientific_rigor_threshold * 0.12
    } else {
        0.0
    };
    let objection_pressure = if segment
        .objection_clusters
        .iter()
        .any(|item| item.to_ascii_lowercase().contains("privacy"))
    {
        buyer.trust_barrier * 0.08
    } else {
        0.0
    };
    clamp01(base + privacy_bonus - rigor_penalty - objection_pressure)
}

fn value_driver_score(
    segment_id: &str,
    driver_ids: &[String],
    drivers_by_id: &BTreeMap<&str, &ValueDriverPrior>,
) -> f64 {
    if driver_ids.is_empty() {
        return 0.55;
    }
    let average_lift = driver_ids
        .iter()
        .filter_map(|driver_id| drivers_by_id.get(driver_id.as_str()).copied())
        .filter_map(|driver| driver.segment_lift.get(segment_id))
        .sum::<f64>()
        / driver_ids.len() as f64;
    clamp01((average_lift - 0.5) / 1.0)
}

fn value_driver_score_for_segment(
    segment: &SegmentBlueprint,
    driver_ids: &[String],
    drivers_by_id: &BTreeMap<&str, &ValueDriverPrior>,
) -> f64 {
    value_driver_score(&segment.segment_id, driver_ids, drivers_by_id)
}

fn buyer_value_driver_score(
    buyer: &BuyerProfile,
    segment: &SegmentBlueprint,
    driver_ids: &[String],
    drivers_by_id: &BTreeMap<&str, &ValueDriverPrior>,
) -> f64 {
    if driver_ids.is_empty() {
        return 0.52;
    }
    let mut total = 0.0;
    for driver_id in driver_ids {
        let base = drivers_by_id
            .get(driver_id.as_str())
            .and_then(|driver| driver.segment_lift.get(&segment.segment_id))
            .copied()
            .unwrap_or(1.0);
        let trait_multiplier = match driver_id.as_str() {
            "weekly_proof" => (buyer.proof_hunger * 0.55) + (buyer.weekly_review_desire * 0.45),
            "few_simple_scores" => {
                (buyer.wearable_ownership * 0.4)
                    + (buyer.passive_data_expectation * 0.4)
                    + ((1.0 - buyer.manual_logging_tolerance) * 0.2)
            }
            "private_exportable_system" => {
                (buyer.privacy_sensitivity * 0.6) + (buyer.trust_barrier * 0.4)
            }
            _ => 0.55,
        };
        total += clamp01((base - 0.4) * 0.7 + (trait_multiplier * 0.6));
    }
    clamp01(total / driver_ids.len() as f64)
}

fn variant_specific_adjustment(buyer: &BuyerProfile, variant: &CampaignVariantDefinition) -> f64 {
    let peptide_coded =
        variant.variant_id.contains("peptide") || variant.variant_id.contains("glp1");
    let natural_coded =
        variant.variant_id.contains("natural") || variant.variant_id.contains("whoop");
    let adjustment = if peptide_coded {
        (buyer.peptide_openness - 0.5) * 0.08
    } else if natural_coded {
        ((1.0 - buyer.peptide_openness) - 0.5) * 0.06
    } else {
        0.0
    };
    adjustment.clamp(-0.06, 0.06)
}

fn simulate_buyer_funnel(
    buyer: &BuyerProfile,
    score: f64,
    channel_fit: f64,
    trust: f64,
    driver: f64,
    penalty: f64,
    rng: &mut StdRng,
) -> BuyerFunnelOutcome {
    let click_probability = clamp01(
        (score * 0.55) + (channel_fit * 0.20) + (buyer.creator_trust * 0.08) + (trust * 0.10)
            - (penalty * 0.10),
    );
    let signup_probability = clamp01(
        click_probability
            * ((score * 0.42) + (driver * 0.22) + (trust * 0.20) + (buyer.proof_hunger * 0.10)
                - (penalty * 0.18)),
    );
    let activation_probability = clamp01(
        signup_probability
            * ((driver * 0.30)
                + (buyer.manual_logging_tolerance * 0.18)
                + (buyer.weekly_review_desire * 0.18)
                + (trust * 0.12)
                - (penalty * 0.16)),
    );
    let retention_probability = clamp01(
        activation_probability
            * ((buyer.subscription_willingness * 0.24)
                + (buyer.weekly_review_desire * 0.18)
                + (driver * 0.18)
                + (trust * 0.10)
                - (penalty * 0.16)),
    );
    let paid_conversion_probability = clamp01(
        retention_probability
            * ((buyer.subscription_willingness * 0.35)
                + (buyer.proof_hunger * 0.10)
                + (trust * 0.08)
                - (penalty * 0.10)),
    );

    let clicked = rng.gen_bool(click_probability);
    let signed_up = clicked && rng.gen_bool(signup_probability);
    let activated = signed_up && rng.gen_bool(activation_probability);
    let retained = activated && rng.gen_bool(retention_probability);
    let converted_paid = retained && rng.gen_bool(paid_conversion_probability);

    BuyerFunnelOutcome {
        click_probability,
        signup_probability,
        activation_probability,
        retention_probability,
        paid_conversion_probability,
        clicked,
        signed_up,
        activated,
        retained,
        converted_paid,
    }
}

fn friction_penalty(
    segment_id: &str,
    friction_ids: &[String],
    frictions_by_id: &BTreeMap<&str, &ProductFrictionPrior>,
) -> f64 {
    if friction_ids.is_empty() {
        return 0.12;
    }
    friction_ids
        .iter()
        .filter_map(|friction_id| frictions_by_id.get(friction_id.as_str()).copied())
        .map(|friction| {
            let modifier = friction
                .segment_modifiers
                .get(segment_id)
                .copied()
                .unwrap_or(1.0);
            let default_impact = friction
                .default_impact
                .as_ref()
                .map(ImpactProfile::average_penalty)
                .unwrap_or(0.1);
            default_impact * modifier
        })
        .sum::<f64>()
        / friction_ids.len() as f64
}

fn friction_penalty_for_segment(
    segment: &SegmentBlueprint,
    friction_ids: &[String],
    frictions_by_id: &BTreeMap<&str, &ProductFrictionPrior>,
) -> f64 {
    friction_penalty(&segment.segment_id, friction_ids, frictions_by_id)
}

fn buyer_friction_penalty(
    buyer: &BuyerProfile,
    segment: &SegmentBlueprint,
    friction_ids: &[String],
    frictions_by_id: &BTreeMap<&str, &ProductFrictionPrior>,
) -> f64 {
    if friction_ids.is_empty() {
        return 0.1;
    }
    let penalty = friction_ids
        .iter()
        .filter_map(|friction_id| frictions_by_id.get(friction_id.as_str()).copied())
        .map(|friction| {
            let modifier = friction
                .segment_modifiers
                .get(&segment.segment_id)
                .copied()
                .unwrap_or(1.0);
            let base = friction
                .default_impact
                .as_ref()
                .map(ImpactProfile::average_penalty)
                .unwrap_or(0.1);
            let trait_pressure = match friction.friction_id.as_str() {
                "manual_logging_burden" => 1.0 - buyer.manual_logging_tolerance,
                "passive_import_gap" => {
                    (buyer.wearable_ownership * 0.45) + (buyer.passive_data_expectation * 0.55)
                }
                "proof_delay" => (buyer.proof_hunger * 0.65) + (buyer.weekly_review_desire * 0.35),
                "trust_gap" => {
                    (buyer.scientific_rigor_threshold * 0.55) + (buyer.trust_barrier * 0.45)
                }
                "privacy_anxiety" => buyer.privacy_sensitivity,
                "category_confusion" => {
                    (buyer.scientific_rigor_threshold * 0.45)
                        + (buyer.proof_hunger * 0.25)
                        + (buyer.dashboard_tolerance * 0.30)
                }
                _ => 0.5,
            };
            base * modifier * trait_pressure
        })
        .sum::<f64>()
        / friction_ids.len() as f64;
    clamp01(penalty)
}

fn aggregate_funnel_metrics(accumulator: &FunnelAccumulator) -> AggregateFunnelMetrics {
    let buyers = accumulator.buyers.max(1);
    AggregateFunnelMetrics {
        buyers: accumulator.buyers,
        clicks: accumulator.clicks,
        signups: accumulator.signups,
        activations: accumulator.activations,
        retained: accumulator.retained,
        paid_conversions: accumulator.paid_conversions,
        click_rate: accumulator.clicks as f64 / buyers as f64,
        signup_rate: accumulator.signups as f64 / buyers as f64,
        activation_rate: accumulator.activations as f64 / buyers as f64,
        retention_rate: accumulator.retained as f64 / buyers as f64,
        paid_conversion_rate: accumulator.paid_conversions as f64 / buyers as f64,
    }
}

fn aggregate_weighted_funnel(segment_scores: &[VariantSegmentScore]) -> AggregateFunnelMetrics {
    if segment_scores.is_empty() {
        return AggregateFunnelMetrics::default();
    }
    let total_weight = segment_scores
        .iter()
        .map(|score| score.effective_weight)
        .sum::<f64>()
        .max(f64::EPSILON);
    let buyers = segment_scores
        .iter()
        .map(|score| score.funnel.buyers)
        .sum::<usize>();
    let clicks = segment_scores
        .iter()
        .map(|score| score.funnel.clicks)
        .sum::<usize>();
    let signups = segment_scores
        .iter()
        .map(|score| score.funnel.signups)
        .sum::<usize>();
    let activations = segment_scores
        .iter()
        .map(|score| score.funnel.activations)
        .sum::<usize>();
    let retained = segment_scores
        .iter()
        .map(|score| score.funnel.retained)
        .sum::<usize>();
    let paid_conversions = segment_scores
        .iter()
        .map(|score| score.funnel.paid_conversions)
        .sum::<usize>();

    AggregateFunnelMetrics {
        buyers,
        clicks,
        signups,
        activations,
        retained,
        paid_conversions,
        click_rate: segment_scores
            .iter()
            .map(|score| score.effective_weight * score.funnel.click_rate)
            .sum::<f64>()
            / total_weight,
        signup_rate: segment_scores
            .iter()
            .map(|score| score.effective_weight * score.funnel.signup_rate)
            .sum::<f64>()
            / total_weight,
        activation_rate: segment_scores
            .iter()
            .map(|score| score.effective_weight * score.funnel.activation_rate)
            .sum::<f64>()
            / total_weight,
        retention_rate: segment_scores
            .iter()
            .map(|score| score.effective_weight * score.funnel.retention_rate)
            .sum::<f64>()
            / total_weight,
        paid_conversion_rate: segment_scores
            .iter()
            .map(|score| score.effective_weight * score.funnel.paid_conversion_rate)
            .sum::<f64>()
            / total_weight,
    }
}

fn aggregate_market_funnel(
    ranked_variants: &[VariantScenarioScore],
    recommended_control: &str,
) -> AggregateFunnelMetrics {
    ranked_variants
        .iter()
        .find(|variant| variant.variant_id == recommended_control)
        .map(|variant| variant.funnel.clone())
        .unwrap_or_default()
}

fn build_observed_data_summary(
    package: &SyntheticMarketPackage,
    scenario: &SyntheticScenarioDefinition,
) -> ObservedDataSummary {
    let observed_for_scenario = package
        .observed_outcomes
        .iter()
        .filter(|outcome| outcome.scenario_id == scenario.scenario_id)
        .collect::<Vec<_>>();

    let usable_records = observed_for_scenario
        .iter()
        .filter(|outcome| observed_outcome_is_usable(outcome))
        .count();
    let placeholder_records = observed_for_scenario.len().saturating_sub(usable_records);
    let total_usable_sample_size = sum_positive_sample_size(
        observed_for_scenario
            .iter()
            .filter(|outcome| observed_outcome_is_usable(outcome))
            .map(|outcome| outcome.sample_size),
    );

    let mut organic_sources = BTreeSet::new();
    let mut paid_sources = BTreeSet::new();
    for outcome in &observed_for_scenario {
        if let Some(source) = outcome
            .source
            .as_ref()
            .filter(|value| !value.trim().is_empty())
        {
            if is_paid_source(source) {
                paid_sources.insert(source.clone());
            } else {
                organic_sources.insert(source.clone());
            }
        }
    }

    let acquisition_motion = if !paid_sources.is_empty() && !organic_sources.is_empty() {
        "mixed"
    } else if !paid_sources.is_empty() {
        "paid_live"
    } else if !observed_for_scenario.is_empty() {
        "organic_only"
    } else {
        "unvalidated"
    }
    .to_string();

    let data_status = if observed_for_scenario.is_empty() {
        "no_observed_data"
    } else if usable_records == 0 {
        "placeholder_only"
    } else if total_usable_sample_size.unwrap_or_default() < 150 || usable_records < 2 {
        "sparse"
    } else {
        "usable"
    }
    .to_string();

    ObservedDataSummary {
        records: observed_for_scenario.len(),
        usable_records,
        placeholder_records,
        total_usable_sample_size,
        organic_sources: organic_sources.into_iter().collect(),
        paid_sources: paid_sources.into_iter().collect(),
        acquisition_motion,
        data_status,
    }
}

fn build_calibration_summary(
    package: &SyntheticMarketPackage,
    scenario: &SyntheticScenarioDefinition,
    ranked_variants: &[VariantScenarioScore],
) -> Vec<SyntheticCalibrationSummary> {
    let observed_for_scenario = package
        .observed_outcomes
        .iter()
        .filter(|outcome| outcome.scenario_id == scenario.scenario_id)
        .collect::<Vec<_>>();

    let mut summaries = Vec::new();
    for variant in ranked_variants {
        let matching = observed_for_scenario
            .iter()
            .copied()
            .filter(|outcome| outcome.variant_id == variant.variant_id)
            .collect::<Vec<_>>();
        if matching.is_empty() {
            continue;
        }
        let usable_matching = matching
            .iter()
            .copied()
            .filter(|outcome| observed_outcome_is_usable(outcome))
            .collect::<Vec<_>>();

        let observed_records = matching.len();
        let usable_observed_records = usable_matching.len();
        let placeholder_records = observed_records.saturating_sub(usable_observed_records);
        let observed_sample_size =
            sum_positive_sample_size(usable_matching.iter().map(|outcome| outcome.sample_size));

        let click_gap = weighted_optional_gap(
            &usable_matching,
            |outcome| outcome.metrics.click_through_rate,
            variant.funnel.click_rate,
        );
        let signup_gap = weighted_optional_gap(
            &usable_matching,
            |outcome| outcome.metrics.signup_rate,
            variant.funnel.signup_rate,
        );
        let activation_gap = weighted_optional_gap(
            &usable_matching,
            |outcome| outcome.metrics.activation_rate,
            variant.funnel.activation_rate,
        );
        let retention_gap = weighted_optional_gap(
            &usable_matching,
            |outcome| outcome.metrics.week_2_retention,
            variant.funnel.retention_rate,
        );
        let paid_conversion_gap = weighted_optional_gap(
            &usable_matching,
            |outcome| outcome.metrics.paid_conversion_rate,
            variant.funnel.paid_conversion_rate,
        );
        let compared_metrics = compared_metric_names(
            click_gap,
            signup_gap,
            activation_gap,
            retention_gap,
            paid_conversion_gap,
        );

        summaries.push(SyntheticCalibrationSummary {
            variant_id: variant.variant_id.clone(),
            observed_records,
            usable_observed_records,
            placeholder_records,
            observed_sample_size,
            compared_metrics,
            click_gap,
            signup_gap,
            activation_gap,
            retention_gap,
            paid_conversion_gap,
            note: calibration_note(
                observed_records,
                usable_observed_records,
                placeholder_records,
                observed_sample_size,
                click_gap,
                signup_gap,
                activation_gap,
                retention_gap,
                paid_conversion_gap,
            ),
        });
    }

    summaries
}

fn weighted_optional_gap(
    observed_outcomes: &[&SyntheticObservedOutcome],
    metric_fn: impl Fn(&SyntheticObservedOutcome) -> Option<f64>,
    simulated_value: f64,
) -> Option<f64> {
    let mut weighted_sum = 0.0;
    let mut total_weight = 0.0;
    for outcome in observed_outcomes {
        let Some(observed_value) = metric_fn(outcome) else {
            continue;
        };
        let weight = positive_sample_size(outcome.sample_size)
            .map(|value| value as f64)
            .unwrap_or(1.0);
        weighted_sum += observed_value * weight;
        total_weight += weight;
    }
    if total_weight <= f64::EPSILON {
        return None;
    }
    let average = weighted_sum / total_weight;
    Some(simulated_value - average)
}

fn compared_metric_names(
    click_gap: Option<f64>,
    signup_gap: Option<f64>,
    activation_gap: Option<f64>,
    retention_gap: Option<f64>,
    paid_conversion_gap: Option<f64>,
) -> Vec<String> {
    let mut metrics = Vec::new();
    for (label, gap) in [
        ("click_through_rate", click_gap),
        ("signup_rate", signup_gap),
        ("activation_rate", activation_gap),
        ("week_2_retention", retention_gap),
        ("paid_conversion_rate", paid_conversion_gap),
    ] {
        if gap.is_some() {
            metrics.push(label.into());
        }
    }
    metrics
}

fn calibration_note(
    observed_records: usize,
    usable_observed_records: usize,
    placeholder_records: usize,
    observed_sample_size: Option<u32>,
    click_gap: Option<f64>,
    signup_gap: Option<f64>,
    activation_gap: Option<f64>,
    retention_gap: Option<f64>,
    paid_conversion_gap: Option<f64>,
) -> String {
    if observed_records == 0 {
        return "No observed outcomes matched this variant yet.".into();
    }
    if usable_observed_records == 0 {
        if placeholder_records > 0 {
            return "Only placeholder outcomes exist for this variant; calibration is not live yet.".into();
        }
        return "Observed outcomes exist for this variant, but no comparable metrics were populated.".into();
    }

    let mut parts = Vec::new();
    for (label, gap) in [
        ("click", click_gap),
        ("signup", signup_gap),
        ("activation", activation_gap),
        ("retention", retention_gap),
        ("paid", paid_conversion_gap),
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
    let confidence = match observed_sample_size.unwrap_or_default() {
        0..=149 => "low",
        150..=499 => "medium",
        _ => "high",
    };
    if placeholder_records > 0 {
        parts.push("placeholders_present".into());
    }
    if parts.is_empty() {
        format!(
            "Calibration has usable records, but no rate metrics overlapped yet (confidence:{confidence})."
        )
    } else {
        format!(
            "Calibration signal {} (confidence:{confidence})",
            parts.join(", ")
        )
    }
}

fn build_business_readiness(
    observed_data_summary: &ObservedDataSummary,
    ranked_variants: &[VariantScenarioScore],
    recommended_control: &str,
) -> BusinessReadinessSummary {
    let control_funnel = aggregate_market_funnel(ranked_variants, recommended_control);
    let organic_readiness_score = as_percent(
        (control_funnel.click_rate * 0.15)
            + (control_funnel.signup_rate * 0.35)
            + (control_funnel.activation_rate * 0.30)
            + (control_funnel.retention_rate * 0.20),
    );
    let mut paid_readiness_base = (control_funnel.signup_rate * 0.20)
        + (control_funnel.activation_rate * 0.35)
        + (control_funnel.retention_rate * 0.30)
        + (control_funnel.paid_conversion_rate * 0.15);
    if observed_data_summary.paid_sources.is_empty() {
        paid_readiness_base *= 0.65;
    }
    let paid_readiness_score = as_percent(paid_readiness_base);
    let subscription_readiness_score = as_percent(
        (control_funnel.activation_rate * 0.25)
            + (control_funnel.retention_rate * 0.50)
            + (control_funnel.paid_conversion_rate * 0.25),
    );

    let mut gating_factors = Vec::new();
    match observed_data_summary.data_status.as_str() {
        "no_observed_data" => {
            gating_factors.push("No observed outcomes yet, so every result is still directional.".into())
        }
        "placeholder_only" => gating_factors.push(
            "Observed outcomes are placeholders only, so calibration has not started yet.".into(),
        ),
        "sparse" => gating_factors.push(
            "Observed outcomes are still sparse, so keep confidence low and learn through repeated tests.".into(),
        ),
        _ => {}
    }
    if observed_data_summary.paid_sources.is_empty() {
        gating_factors.push(
            "No paid traffic data yet, so paid-readiness is inferred from funnel quality rather than validated CAC.".into(),
        );
    }
    if control_funnel.activation_rate < 0.16 {
        gating_factors
            .push("Activation is still too soft to support efficient acquisition at scale.".into());
    }
    if control_funnel.retention_rate < 0.08 {
        gating_factors.push(
            "Early repeat behavior is still too weak to treat this as a compounding growth system."
                .into(),
        );
    }
    if control_funnel.paid_conversion_rate < 0.03 {
        gating_factors.push(
            "Monetization intent remains weak or unvalidated, so the revenue model still needs proof.".into(),
        );
    }

    let current_focus = if observed_data_summary.paid_sources.is_empty()
        && (paid_readiness_score < 60
            || control_funnel.activation_rate < 0.20
            || control_funnel.retention_rate < 0.10)
    {
        "Stay focused on organic channels, wedge clarity, and repeatable conversion proof before any paid scale."
    } else if paid_readiness_score < 60 {
        "Keep running low-cost content and relationship loops until the control motion is more efficient."
    } else {
        "The top motion looks strong enough for tightly capped paid tests."
    }
    .into();

    BusinessReadinessSummary {
        acquisition_motion: observed_data_summary.acquisition_motion.clone(),
        observed_data_status: observed_data_summary.data_status.clone(),
        organic_readiness_score,
        paid_readiness_score,
        subscription_readiness_score,
        current_focus,
        gating_factors,
    }
}

fn observed_outcome_has_metrics(outcome: &SyntheticObservedOutcome) -> bool {
    [
        outcome.metrics.click_through_rate,
        outcome.metrics.signup_rate,
        outcome.metrics.activation_rate,
        outcome.metrics.week_2_retention,
        outcome.metrics.paid_conversion_rate,
        outcome.metrics.share_rate,
    ]
    .into_iter()
    .any(|value| value.is_some())
}

fn positive_sample_size(sample_size: Option<u32>) -> Option<u32> {
    sample_size.filter(|value| *value > 0)
}

fn observed_outcome_is_usable(outcome: &SyntheticObservedOutcome) -> bool {
    observed_outcome_has_metrics(outcome)
        && outcome.sample_size.map(|value| value > 0).unwrap_or(true)
}

fn sum_positive_sample_size(sample_sizes: impl Iterator<Item = Option<u32>>) -> Option<u32> {
    let total = sample_sizes
        .filter_map(positive_sample_size)
        .fold(0u32, |acc, value| acc.saturating_add(value));
    (total > 0).then_some(total)
}

fn is_paid_source(source: &str) -> bool {
    let normalized = source.trim().to_ascii_lowercase();
    [
        "paid",
        "ads",
        "cpc",
        "facebook_ads",
        "meta_ads",
        "google_ads",
        "tiktok_ads",
        "youtube_ads",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn role_bonus(role: Option<&str>) -> f64 {
    match role {
        Some("control") => 0.03,
        Some("challenger") => 0.02,
        Some("trust_layer") => -0.01,
        Some("long_term_narrative") => -0.03,
        Some("niche_expansion") => -0.02,
        _ => 0.0,
    }
}

fn build_risk_flags(
    variant: &CampaignVariantDefinition,
    scenario_channels: &[String],
) -> Vec<String> {
    let mut flags = Vec::new();
    let overlap = channel_overlap_score(scenario_channels, &variant.channel_fit);
    if overlap < 0.34 {
        flags.push("low_channel_overlap".into());
    }
    if variant
        .core_risks
        .iter()
        .any(|risk| risk.to_ascii_lowercase().contains("logging"))
    {
        flags.push("logging_risk".into());
    }
    if variant
        .core_risks
        .iter()
        .any(|risk| risk.to_ascii_lowercase().contains("broad"))
    {
        flags.push("broad_story_risk".into());
    }
    if variant
        .core_risks
        .iter()
        .any(|risk| risk.to_ascii_lowercase().contains("narrow"))
    {
        flags.push("narrow_tam_risk".into());
    }
    flags
}

fn build_simulation_notes(
    scenario: &SyntheticScenarioDefinition,
    segment_summaries: &[SegmentScenarioSummary],
    observed_data_summary: &ObservedDataSummary,
    business_readiness: &BusinessReadinessSummary,
) -> Vec<String> {
    let mut notes = Vec::new();
    if let Some(top_segment) = segment_summaries.first() {
        notes.push(format!(
            "Strongest current segment winner in this scenario is {} via {}.",
            top_segment.segment_id, top_segment.best_variant_id
        ));
    }
    notes.push(format!(
        "Scenario {} compares {} variants across {} channels.",
        scenario.scenario_id,
        scenario.campaign_variants.len(),
        scenario.channels.len()
    ));
    notes.push(format!(
        "Observed data status is {} with {} usable records; acquisition motion is {}.",
        observed_data_summary.data_status,
        observed_data_summary.usable_records,
        observed_data_summary.acquisition_motion
    ));
    notes.push(format!(
        "Current focus: {}",
        business_readiness.current_focus
    ));
    notes
}

fn default_distribution_name() -> String {
    "beta".into()
}

fn clamp01(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}

fn as_percent(value: f64) -> u32 {
    (clamp01(value) * 100.0).round() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_package() -> SyntheticMarketPackage {
        SyntheticMarketPackage {
            market: SyntheticMarketMetadata {
                name: "Mirrorlife".into(),
                version: Some("0.1.0".into()),
                currency: Some("USD".into()),
                region: Some("US".into()),
                pricing_reference: BTreeMap::new(),
            },
            segments: vec![
                SegmentBlueprint {
                    segment_id: "glp1".into(),
                    name: "GLP-1".into(),
                    priority: Some("primary".into()),
                    share_prior: 0.6,
                    adjacent_to: vec!["privacy".into()],
                    description: None,
                    demographic_shape: BTreeMap::new(),
                    jobs: vec![],
                    traits: BTreeMap::new(),
                    preferred_channels: vec![],
                    objection_clusters: vec![],
                },
                SegmentBlueprint {
                    segment_id: "privacy".into(),
                    name: "Privacy".into(),
                    priority: Some("secondary".into()),
                    share_prior: 0.4,
                    adjacent_to: vec!["glp1".into()],
                    description: None,
                    demographic_shape: BTreeMap::new(),
                    jobs: vec![],
                    traits: BTreeMap::new(),
                    preferred_channels: vec![],
                    objection_clusters: vec![],
                },
            ],
            overlap_assumptions: vec![SegmentOverlapAssumption {
                from_segment: "glp1".into(),
                to_segment: "privacy".into(),
                overlap_score: 0.3,
            }],
            frictions: vec![ProductFrictionPrior {
                friction_id: "logging".into(),
                label: "Logging".into(),
                description: None,
                default_impact: Some(ImpactProfile {
                    signup_penalty: 0.08,
                    activation_penalty: 0.18,
                    retention_penalty: 0.22,
                }),
                segment_modifiers: BTreeMap::from([("glp1".into(), 1.0), ("privacy".into(), 1.2)]),
            }],
            value_drivers: vec![ValueDriverPrior {
                driver_id: "proof".into(),
                label: "Proof".into(),
                description: None,
                segment_lift: BTreeMap::from([("glp1".into(), 1.3), ("privacy".into(), 0.8)]),
            }],
            channels: vec![ChannelAssumption {
                channel_id: "landing_page".into(),
                role: Some("conversion".into()),
                fit_notes: vec![],
                reach_priority: Some("high".into()),
                trust_base: 0.7,
                creator_lift: 0.0,
                friction_tolerance: 0.5,
            }],
            campaign_variants: vec![CampaignVariantDefinition {
                variant_id: "proof_wedge".into(),
                role: Some("control".into()),
                summary: Some("Track what you're taking".into()),
                best_for_segments: vec!["glp1".into()],
                channel_fit: vec!["landing_page".into()],
                supports_value_drivers: vec!["proof".into()],
                exposed_frictions: vec!["logging".into()],
                core_strengths: vec![],
                core_risks: vec!["logging burden".into()],
            }],
            scenarios: vec![SyntheticScenarioDefinition {
                scenario_id: "wedge_test".into(),
                goal: "Find best wedge".into(),
                decision: "Pick control".into(),
                primary_segments: vec!["glp1".into()],
                secondary_segments: vec!["privacy".into()],
                campaign_variants: vec!["proof_wedge".into()],
                channels: vec!["landing_page".into()],
                key_questions: vec!["who converts".into()],
                success_metrics: vec!["signup_rate".into()],
            }],
            observed_outcomes: vec![],
        }
    }

    #[test]
    fn synthetic_market_package_validates() {
        let package = sample_package();
        assert_eq!(package.validate(), Ok(()));
    }

    #[test]
    fn synthetic_market_package_requires_share_total_near_one() {
        let mut package = sample_package();
        package.segments[0].share_prior = 0.2;
        assert_eq!(
            package.validate(),
            Err(SyntheticMarketValidationError::InvalidSegmentShareTotal)
        );
    }

    #[test]
    fn synthetic_market_package_rejects_unknown_variant_channel() {
        let mut package = sample_package();
        package.campaign_variants[0].channel_fit = vec!["tiktok".into()];
        assert_eq!(
            package.validate(),
            Err(SyntheticMarketValidationError::UnknownVariantChannel)
        );
    }

    #[test]
    fn synthetic_market_package_rejects_unknown_scenario_variant() {
        let mut package = sample_package();
        package.scenarios[0].campaign_variants = vec!["missing".into()];
        assert_eq!(
            package.validate(),
            Err(SyntheticMarketValidationError::UnknownScenarioVariant)
        );
    }

    #[test]
    fn simulate_synthetic_market_returns_ranked_variant_output() {
        let package = sample_package();
        let result = simulate_synthetic_market(&package, "wedge_test").unwrap();
        assert_eq!(result.recommended_control, "proof_wedge");
        assert_eq!(result.ranked_variants.len(), 1);
        assert_eq!(result.segment_summaries.len(), 2);
        assert_eq!(result.observed_data_summary.data_status, "no_observed_data");
    }

    #[test]
    fn placeholder_observed_outcomes_do_not_count_as_usable_calibration() {
        let mut package = sample_package();
        package.observed_outcomes = vec![SyntheticObservedOutcome {
            experiment_id: "placeholder".into(),
            scenario_id: "wedge_test".into(),
            variant_id: "proof_wedge".into(),
            segment_id: Some("glp1".into()),
            channel: Some("landing_page".into()),
            source: Some("organic_waitlist".into()),
            creative_id: None,
            hook_id: None,
            landing_variant: None,
            sample_size: Some(0),
            metrics: SyntheticObservedMetrics::default(),
            notes: None,
        }];

        let result = simulate_synthetic_market(&package, "wedge_test").unwrap();
        assert_eq!(
            result.observed_data_summary.acquisition_motion,
            "organic_only"
        );
        assert_eq!(result.observed_data_summary.data_status, "placeholder_only");
        assert_eq!(result.calibration_summary.len(), 1);
        assert_eq!(result.calibration_summary[0].usable_observed_records, 0);
    }
}
