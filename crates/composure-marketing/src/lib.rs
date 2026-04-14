use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use composure_core::{
    analyze_composure_checked, run_scenario_monte_carlo_checked, summarize_run, Action, ActionType,
    Archetype, MonteCarloConfig, RunSummary, Scenario, SimState, Simulator,
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketingSimulationRequest {
    pub seed_data: SeedData,
    pub approaches: Vec<ApproachInput>,
    #[serde(default = "default_simulation_size")]
    pub simulation_size: usize,
    #[serde(default)]
    pub platforms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedData {
    #[serde(default)]
    pub personas: Vec<PersonaSeed>,
    #[serde(default)]
    pub competitors: Vec<String>,
    pub project_name: String,
    pub project_description: String,
    #[serde(default)]
    pub platform_context: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaSeed {
    pub name: String,
    #[serde(rename = "type")]
    pub persona_type: String,
    #[serde(default)]
    pub demographics: Option<Value>,
    #[serde(default)]
    pub psychographics: Option<Value>,
    #[serde(default)]
    pub relationship: Option<String>,
    #[serde(default)]
    pub preferences: Vec<String>,
    #[serde(default)]
    pub objections: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApproachInput {
    pub id: String,
    pub angle: String,
    pub format: String,
    #[serde(default)]
    pub channels: Vec<String>,
    pub tone: String,
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketingSimulationRequestV2 {
    pub project: ProjectContext,
    #[serde(default)]
    pub personas: Vec<PersonaDefinition>,
    #[serde(default)]
    pub approaches: Vec<ApproachDefinition>,
    #[serde(default)]
    pub channels: Vec<ChannelContext>,
    #[serde(default)]
    pub audience_weighting: Vec<AudienceWeighting>,
    #[serde(default)]
    pub scenario: ScenarioDefinition,
    #[serde(default)]
    pub output: OutputOptions,
    #[serde(default = "default_simulation_size")]
    pub simulation_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub competitors: Vec<String>,
    #[serde(default)]
    pub platform_context: Vec<String>,
    #[serde(default)]
    pub constraints: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaDefinition {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub persona_type: String,
    #[serde(default)]
    pub demographics: Option<Value>,
    #[serde(default)]
    pub psychographics: Option<Value>,
    #[serde(default)]
    pub relationship: Option<String>,
    #[serde(default)]
    pub jobs: Vec<String>,
    #[serde(default)]
    pub preferences: Vec<String>,
    #[serde(default)]
    pub objections: Vec<String>,
    #[serde(default)]
    pub channels: Vec<String>,
    #[serde(default)]
    pub conversion_barriers: Vec<String>,
    #[serde(default)]
    pub trust_signals: Vec<String>,
    #[serde(default)]
    pub price_sensitivity: Option<f64>,
    #[serde(default)]
    pub proof_threshold: Option<f64>,
    #[serde(default)]
    pub privacy_sensitivity: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApproachDefinition {
    pub id: String,
    pub angle: String,
    pub format: String,
    #[serde(default)]
    pub channels: Vec<String>,
    pub tone: String,
    pub target: String,
    #[serde(default)]
    pub objectives: Vec<ObjectiveDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelContext {
    pub channel: String,
    #[serde(default)]
    pub norms: Vec<String>,
    #[serde(default)]
    pub constraints: Vec<String>,
    #[serde(default = "default_relative_weight")]
    pub relative_weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectiveDefinition {
    pub metric: MetricKind,
    #[serde(default = "default_objective_weight")]
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudienceWeighting {
    pub persona_id: String,
    #[serde(default = "default_objective_weight")]
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioDefinition {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub scenario_type: ScenarioType,
    #[serde(default = "default_time_steps")]
    pub time_steps: usize,
    #[serde(default)]
    pub objectives: Vec<ObjectiveDefinition>,
}

impl Default for ScenarioDefinition {
    fn default() -> Self {
        Self {
            name: "default".into(),
            description: None,
            scenario_type: ScenarioType::AudienceDiscovery,
            time_steps: default_time_steps(),
            objectives: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputOptions {
    #[serde(default = "default_true")]
    pub include_persona_breakdown: bool,
    #[serde(default = "default_true")]
    pub include_metric_breakdown: bool,
    #[serde(default = "default_true")]
    pub include_mean_trajectory: bool,
}

impl Default for OutputOptions {
    fn default() -> Self {
        Self {
            include_persona_breakdown: true,
            include_metric_breakdown: true,
            include_mean_trajectory: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioType {
    AudienceDiscovery,
    Positioning,
    CampaignSequence,
    CommunityActivation,
    Retention,
    Custom,
}

impl Default for ScenarioType {
    fn default() -> Self {
        Self::AudienceDiscovery
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum MetricKind {
    AudienceReceptivity,
    PersonaFit,
    ChannelFit,
    MessageClarity,
    ConversionIntent,
    Shareability,
    TrustSignal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketingSimulationResult {
    pub simulation_id: String,
    pub approach_results: Vec<ApproachSimulationResult>,
    pub cross_approach_insights: Vec<String>,
    pub engine: EngineMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineMetadata {
    pub name: String,
    pub version: String,
    pub model: String,
    pub seed_base: u64,
    pub time_steps: usize,
    pub num_paths: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApproachSimulationResult {
    pub approach_id: String,
    pub engagement_score: u32,
    pub viral_potential: u32,
    pub sentiment_distribution: SentimentDistribution,
    pub emergent_behaviors: Vec<String>,
    pub top_reactions: Vec<String>,
    pub concerns: Vec<String>,
    pub composure_archetype: String,
    pub run_summary: RunSummary,
    pub mean_trajectory: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketingSimulationResultV2 {
    pub simulation_id: String,
    pub scenario: ScenarioDefinition,
    pub primary_scorecard: PrimaryScorecard,
    pub approach_results: Vec<ApproachSimulationResultV2>,
    pub cross_approach_insights: Vec<String>,
    pub engine: EngineMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApproachSimulationResultV2 {
    pub approach_id: String,
    pub primary_scorecard: PrimaryScorecard,
    pub engagement_score: u32,
    pub viral_potential: u32,
    pub sentiment_distribution: SentimentDistribution,
    pub persona_results: Vec<PersonaApproachResult>,
    pub emergent_behaviors: Vec<String>,
    pub top_reactions: Vec<String>,
    pub concerns: Vec<String>,
    pub composure_archetype: String,
    pub run_summary: RunSummary,
    pub mean_trajectory: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimaryScorecard {
    pub overall_score: u32,
    pub metrics: Vec<MetricScore>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricScore {
    pub metric: MetricKind,
    pub label: String,
    pub score: u32,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaApproachResult {
    pub persona_id: String,
    pub persona_name: String,
    pub audience_weight: f64,
    pub primary_scorecard: PrimaryScorecard,
    pub engagement_score: u32,
    pub viral_potential: u32,
    pub sentiment_distribution: SentimentDistribution,
    pub top_reactions: Vec<String>,
    pub concerns: Vec<String>,
    pub composure_archetype: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentimentDistribution {
    pub positive: u32,
    pub neutral: u32,
    pub negative: u32,
}

#[derive(Debug, Error)]
pub enum MarketingSimulationError {
    #[error("project_name cannot be empty")]
    EmptyProjectName,
    #[error("at least one approach is required")]
    MissingApproaches,
    #[error("persona ID cannot be empty")]
    EmptyPersonaId,
    #[error("approach ID cannot be empty")]
    EmptyApproachId,
    #[error("simulation_size must be greater than zero")]
    InvalidSimulationSize,
    #[error("time_steps must be greater than zero")]
    InvalidTimeSteps,
    #[error("scenario error: {0}")]
    Scenario(#[from] composure_core::ScenarioError),
    #[error("Monte Carlo error: {0}")]
    MonteCarlo(#[from] composure_core::MonteCarloError),
    #[error("composure error: {0}")]
    Composure(#[from] composure_core::ComposureError),
}

#[derive(Debug, Clone)]
struct AudienceProfile {
    preference_fit: f64,
    objection_risk: f64,
    persona_focus: f64,
    relationship_pull: f64,
    competitor_pressure: f64,
    platform_alignment: f64,
    channel_focus: f64,
    format_strength: f64,
    tone_conviction: f64,
    novelty: f64,
    specificity: f64,
}

#[derive(Debug, Clone)]
struct MarketingSimulator {
    profile: AudienceProfile,
    dynamics: ScenarioDynamics,
}

#[derive(Debug, Clone)]
struct ApproachComputation {
    profile: AudienceProfile,
    final_means: Vec<f64>,
    run_summary: RunSummary,
    engagement_score: u32,
    viral_potential: u32,
    sentiment_distribution: SentimentDistribution,
    emergent_behaviors: Vec<String>,
    top_reactions: Vec<String>,
    concerns: Vec<String>,
    composure_archetype: String,
    mean_trajectory: Vec<f64>,
}

#[derive(Debug, Clone)]
struct ScenarioDynamics {
    initial_state_bias: [f64; 3],
    initial_memory_shift: [f64; 3],
    initial_uncertainty_shift: [f64; 3],
    action_effect_multipliers: [f64; 3],
    action_decay: [f64; 3],
    memory_penalty_multipliers: [f64; 3],
    uncertainty_penalty_multipliers: [f64; 3],
    memory_decay: [f64; 3],
    uncertainty_decay: [f64; 3],
    resonance_to_attention: f64,
    share_to_attention: f64,
    attention_to_resonance: f64,
    relationship_to_resonance: f64,
    attention_to_share: f64,
    resonance_to_share: f64,
    health_weights: [f64; 3],
}

pub fn simulate_marketing(
    request: &MarketingSimulationRequest,
) -> Result<MarketingSimulationResult, MarketingSimulationError> {
    validate_request(request)?;

    let simulation_id = build_simulation_id(request);
    let seed_base = derive_seed_base(request, &simulation_id);
    let time_steps = default_time_steps();

    let approach_results = request
        .approaches
        .iter()
        .enumerate()
        .map(|(index, approach)| {
            let computation = simulate_approach(
                &request.seed_data,
                approach,
                &request.platforms,
                request.simulation_size,
                seed_base.wrapping_add((index as u64) * 97),
                index,
                time_steps,
                &ScenarioType::AudienceDiscovery,
            )?;

            Ok(ApproachSimulationResult {
                approach_id: approach.id.clone(),
                engagement_score: computation.engagement_score,
                viral_potential: computation.viral_potential,
                sentiment_distribution: computation.sentiment_distribution,
                emergent_behaviors: computation.emergent_behaviors,
                top_reactions: computation.top_reactions,
                concerns: computation.concerns,
                composure_archetype: computation.composure_archetype,
                run_summary: computation.run_summary,
                mean_trajectory: computation.mean_trajectory,
            })
        })
        .collect::<Result<Vec<_>, MarketingSimulationError>>()?;

    let cross_approach_insights =
        build_cross_approach_insights(&request.approaches, &approach_results);

    Ok(MarketingSimulationResult {
        simulation_id,
        approach_results,
        cross_approach_insights,
        engine: EngineMetadata {
            name: "composure-marketing".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            model: "deterministic_marketing_adapter_v1".into(),
            seed_base,
            time_steps,
            num_paths: request.simulation_size,
        },
    })
}

pub fn simulate_marketing_v2(
    request: &MarketingSimulationRequestV2,
) -> Result<MarketingSimulationResultV2, MarketingSimulationError> {
    validate_v2_request(request)?;

    let simulation_id = build_simulation_id_v2(request);
    let seed_base = derive_seed_base_v2(request, &simulation_id);
    let seed_data =
        project_context_to_seed_data(&request.project, &request.personas, &request.channels);
    let platforms = v2_platforms(request);
    let time_steps = request.scenario.time_steps.max(1);

    let approach_results = request
        .approaches
        .iter()
        .enumerate()
        .map(|(index, approach)| {
            let approach_input = approach_definition_to_input(approach.clone());
            let aggregate = simulate_approach(
                &seed_data,
                &approach_input,
                &platforms,
                request.simulation_size,
                seed_base.wrapping_add((index as u64) * 97),
                index,
                time_steps,
                &request.scenario.scenario_type,
            )?;

            let persona_results = if request.output.include_persona_breakdown {
                request
                    .personas
                    .iter()
                    .enumerate()
                    .map(|(persona_index, persona)| {
                        let persona_seed_data = project_context_to_seed_data(
                            &request.project,
                            std::slice::from_ref(persona),
                            &request.channels,
                        );
                        let persona_computation = simulate_approach(
                            &persona_seed_data,
                            &approach_input,
                            &platforms,
                            request.simulation_size,
                            seed_base
                                .wrapping_add((index as u64) * 97)
                                .wrapping_add((persona_index as u64 + 1) * 131),
                            index,
                            time_steps,
                            &request.scenario.scenario_type,
                        )?;

                        Ok(PersonaApproachResult {
                            persona_id: persona.id.clone(),
                            persona_name: persona.name.clone(),
                            audience_weight: audience_weight_for(
                                &request.audience_weighting,
                                &persona.id,
                            ),
                            primary_scorecard: build_primary_scorecard(
                                &persona_computation.profile,
                                &persona_computation.final_means,
                                persona_computation.engagement_score,
                                persona_computation.viral_potential,
                                &resolve_objectives(request, approach),
                                &request.scenario.scenario_type,
                                Some(persona),
                                approach,
                                request.output.include_metric_breakdown,
                            ),
                            engagement_score: persona_computation.engagement_score,
                            viral_potential: persona_computation.viral_potential,
                            sentiment_distribution: persona_computation.sentiment_distribution,
                            top_reactions: persona_computation.top_reactions,
                            concerns: persona_computation.concerns,
                            composure_archetype: persona_computation.composure_archetype,
                        })
                    })
                    .collect::<Result<Vec<_>, MarketingSimulationError>>()?
            } else {
                Vec::new()
            };

            let mut primary_scorecard = build_primary_scorecard(
                &aggregate.profile,
                &aggregate.final_means,
                aggregate.engagement_score,
                aggregate.viral_potential,
                &resolve_objectives(request, approach),
                &request.scenario.scenario_type,
                None,
                approach,
                request.output.include_metric_breakdown,
            );
            if !persona_results.is_empty() {
                primary_scorecard = blend_persona_scores(
                    primary_scorecard,
                    &persona_results,
                    request.output.include_metric_breakdown,
                );
            }

            Ok(ApproachSimulationResultV2 {
                approach_id: approach.id.clone(),
                primary_scorecard,
                engagement_score: aggregate.engagement_score,
                viral_potential: aggregate.viral_potential,
                sentiment_distribution: aggregate.sentiment_distribution,
                persona_results,
                emergent_behaviors: aggregate.emergent_behaviors,
                top_reactions: aggregate.top_reactions,
                concerns: aggregate.concerns,
                composure_archetype: aggregate.composure_archetype,
                run_summary: aggregate.run_summary,
                mean_trajectory: if request.output.include_mean_trajectory {
                    aggregate.mean_trajectory
                } else {
                    Vec::new()
                },
            })
        })
        .collect::<Result<Vec<_>, MarketingSimulationError>>()?;

    let cross_approach_insights =
        build_cross_approach_insights_v2(&request.approaches, &approach_results);

    Ok(MarketingSimulationResultV2 {
        simulation_id,
        scenario: request.scenario.clone(),
        primary_scorecard: summarize_v2_scorecard(
            &approach_results,
            request.output.include_metric_breakdown,
        ),
        approach_results,
        cross_approach_insights,
        engine: EngineMetadata {
            name: "composure-marketing".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            model: "deterministic_marketing_adapter_v2_scaffold".into(),
            seed_base,
            time_steps,
            num_paths: request.simulation_size,
        },
    })
}

fn validate_request(request: &MarketingSimulationRequest) -> Result<(), MarketingSimulationError> {
    if request.seed_data.project_name.trim().is_empty() {
        return Err(MarketingSimulationError::EmptyProjectName);
    }
    if request.approaches.is_empty() {
        return Err(MarketingSimulationError::MissingApproaches);
    }
    if request.simulation_size == 0 {
        return Err(MarketingSimulationError::InvalidSimulationSize);
    }
    if request
        .approaches
        .iter()
        .any(|approach| approach.id.trim().is_empty())
    {
        return Err(MarketingSimulationError::EmptyApproachId);
    }
    Ok(())
}

fn validate_v2_request(
    request: &MarketingSimulationRequestV2,
) -> Result<(), MarketingSimulationError> {
    let mapped = MarketingSimulationRequest {
        seed_data: project_context_to_seed_data(
            &request.project,
            &request.personas,
            &request.channels,
        ),
        approaches: request
            .approaches
            .iter()
            .cloned()
            .map(approach_definition_to_input)
            .collect(),
        simulation_size: request.simulation_size,
        platforms: v2_platforms(request),
    };
    validate_request(&mapped)?;

    if request
        .personas
        .iter()
        .any(|persona| persona.id.trim().is_empty())
    {
        return Err(MarketingSimulationError::EmptyPersonaId);
    }
    if request.scenario.time_steps == 0 {
        return Err(MarketingSimulationError::InvalidTimeSteps);
    }

    Ok(())
}

fn simulate_approach(
    seed_data: &SeedData,
    approach: &ApproachInput,
    platforms: &[String],
    simulation_size: usize,
    seed: u64,
    index: usize,
    time_steps: usize,
    scenario_type: &ScenarioType,
) -> Result<ApproachComputation, MarketingSimulationError> {
    let profile = build_audience_profile(seed_data, approach, platforms);
    let dynamics = scenario_dynamics(scenario_type);
    let scenario = build_scenario(index, approach, &profile, time_steps, scenario_type, &dynamics);
    let simulator = MarketingSimulator {
        profile: profile.clone(),
        dynamics,
    };
    let config = MonteCarloConfig::with_seed(simulation_size, scenario.time_steps, seed);
    let monte_carlo = run_scenario_monte_carlo_checked(&simulator, &scenario, &config, true)?;
    let composure = analyze_composure_checked(
        &monte_carlo.mean_trajectory,
        scenario.failure_threshold.unwrap_or(0.45),
    )?;
    let run_summary = summarize_run(Some(&monte_carlo), Some(&composure));
    let final_means = mean_final_state(&monte_carlo);
    let engagement_score = compute_engagement_score(&run_summary, &final_means);
    let viral_potential = compute_viral_potential(&run_summary, &final_means, &profile);
    let sentiment_distribution =
        derive_sentiment_distribution(&final_means, &profile, engagement_score);
    let concerns = build_concerns(approach, &profile);
    let top_reactions = build_top_reactions(approach, &profile);
    let emergent_behaviors = build_emergent_behaviors(
        approach,
        composure.archetype,
        engagement_score,
        viral_potential,
        &profile,
    );

    Ok(ApproachComputation {
        profile,
        final_means,
        run_summary,
        engagement_score,
        viral_potential,
        sentiment_distribution,
        emergent_behaviors,
        top_reactions,
        concerns,
        composure_archetype: composure.archetype.label().to_string(),
        mean_trajectory: monte_carlo.mean_trajectory,
    })
}

fn project_context_to_seed_data(
    project: &ProjectContext,
    personas: &[PersonaDefinition],
    channels: &[ChannelContext],
) -> SeedData {
    let mut platform_context = project.platform_context.clone();
    for channel in channels {
        platform_context.push(channel.channel.clone());
        platform_context.extend(channel.norms.iter().cloned());
        platform_context.extend(channel.constraints.iter().cloned());
    }
    if let Some(category) = &project.category {
        platform_context.push(category.clone());
    }
    platform_context.extend(project.constraints.iter().cloned());
    platform_context.extend(project.tags.iter().cloned());

    SeedData {
        personas: personas
            .iter()
            .cloned()
            .map(persona_definition_to_seed)
            .collect(),
        competitors: project.competitors.clone(),
        project_name: project.name.clone(),
        project_description: project.description.clone(),
        platform_context,
    }
}

fn persona_definition_to_seed(persona: PersonaDefinition) -> PersonaSeed {
    PersonaSeed {
        name: persona.name,
        persona_type: persona.persona_type,
        demographics: persona.demographics,
        psychographics: persona.psychographics,
        relationship: persona.relationship,
        preferences: persona
            .preferences
            .into_iter()
            .chain(persona.jobs)
            .chain(persona.trust_signals)
            .collect(),
        objections: persona
            .objections
            .into_iter()
            .chain(persona.conversion_barriers)
            .collect(),
    }
}

fn approach_definition_to_input(approach: ApproachDefinition) -> ApproachInput {
    ApproachInput {
        id: approach.id,
        angle: approach.angle,
        format: approach.format,
        channels: approach.channels,
        tone: approach.tone,
        target: approach.target,
    }
}

fn v2_platforms(request: &MarketingSimulationRequestV2) -> Vec<String> {
    let mut platforms = request
        .channels
        .iter()
        .map(|channel| channel.channel.clone())
        .collect::<Vec<_>>();
    for approach in &request.approaches {
        platforms.extend(approach.channels.iter().cloned());
    }
    if platforms.is_empty() {
        platforms.extend(request.project.platform_context.iter().cloned());
    }
    platforms
}

fn audience_weight_for(weights: &[AudienceWeighting], persona_id: &str) -> f64 {
    weights
        .iter()
        .find(|weight| weight.persona_id == persona_id)
        .map(|weight| weight.weight.max(0.0))
        .unwrap_or(1.0)
}

fn resolve_objectives(
    request: &MarketingSimulationRequestV2,
    approach: &ApproachDefinition,
) -> Vec<ObjectiveDefinition> {
    if !approach.objectives.is_empty() {
        approach.objectives.clone()
    } else if !request.scenario.objectives.is_empty() {
        request.scenario.objectives.clone()
    } else {
        default_objectives_for(&request.scenario.scenario_type)
    }
}

fn default_objectives_for(scenario_type: &ScenarioType) -> Vec<ObjectiveDefinition> {
    let metrics = match scenario_type {
        ScenarioType::AudienceDiscovery | ScenarioType::Positioning => vec![
            (MetricKind::AudienceReceptivity, 0.28),
            (MetricKind::PersonaFit, 0.22),
            (MetricKind::MessageClarity, 0.18),
            (MetricKind::ChannelFit, 0.14),
            (MetricKind::ConversionIntent, 0.18),
        ],
        ScenarioType::CampaignSequence => vec![
            (MetricKind::ConversionIntent, 0.28),
            (MetricKind::AudienceReceptivity, 0.22),
            (MetricKind::Shareability, 0.20),
            (MetricKind::TrustSignal, 0.15),
            (MetricKind::ChannelFit, 0.15),
        ],
        ScenarioType::CommunityActivation => vec![
            (MetricKind::Shareability, 0.28),
            (MetricKind::TrustSignal, 0.20),
            (MetricKind::PersonaFit, 0.20),
            (MetricKind::AudienceReceptivity, 0.18),
            (MetricKind::ChannelFit, 0.14),
        ],
        ScenarioType::Retention => vec![
            (MetricKind::TrustSignal, 0.28),
            (MetricKind::ConversionIntent, 0.24),
            (MetricKind::PersonaFit, 0.18),
            (MetricKind::AudienceReceptivity, 0.16),
            (MetricKind::MessageClarity, 0.14),
        ],
        ScenarioType::Custom => vec![
            (MetricKind::AudienceReceptivity, 0.25),
            (MetricKind::PersonaFit, 0.15),
            (MetricKind::ChannelFit, 0.15),
            (MetricKind::MessageClarity, 0.15),
            (MetricKind::ConversionIntent, 0.15),
            (MetricKind::Shareability, 0.15),
        ],
    };

    metrics
        .into_iter()
        .map(|(metric, weight)| ObjectiveDefinition { metric, weight })
        .collect()
}

fn build_primary_scorecard(
    profile: &AudienceProfile,
    final_means: &[f64],
    engagement_score: u32,
    viral_potential: u32,
    objectives: &[ObjectiveDefinition],
    scenario_type: &ScenarioType,
    persona: Option<&PersonaDefinition>,
    approach: &ApproachDefinition,
    include_metric_breakdown: bool,
) -> PrimaryScorecard {
    let metrics = build_metric_scores(
        profile,
        final_means,
        engagement_score,
        viral_potential,
        scenario_type,
        persona,
        approach,
    );
    let overall_score = weighted_metric_average(&metrics, objectives);

    PrimaryScorecard {
        overall_score,
        metrics: if include_metric_breakdown {
            metrics
        } else {
            Vec::new()
        },
    }
}

fn build_metric_scores(
    profile: &AudienceProfile,
    final_means: &[f64],
    engagement_score: u32,
    viral_potential: u32,
    scenario_type: &ScenarioType,
    persona: Option<&PersonaDefinition>,
    approach: &ApproachDefinition,
) -> Vec<MetricScore> {
    let mut metrics = vec![
        MetricScore {
            metric: MetricKind::AudienceReceptivity,
            label: metric_label(&MetricKind::AudienceReceptivity),
            score: engagement_score,
            explanation: "Derived from Monte Carlo trajectory strength and end-state resonance."
                .into(),
        },
        MetricScore {
            metric: MetricKind::PersonaFit,
            label: metric_label(&MetricKind::PersonaFit),
            score: as_percent(
                (profile.preference_fit * 0.50)
                    + (profile.persona_focus * 0.30)
                    + (profile.specificity * 0.20),
            ),
            explanation:
                "Measures how specifically the message matches seeded audience motivations.".into(),
        },
        MetricScore {
            metric: MetricKind::ChannelFit,
            label: metric_label(&MetricKind::ChannelFit),
            score: as_percent(
                (profile.platform_alignment * 0.55)
                    + (profile.channel_focus * 0.20)
                    + (profile.format_strength * 0.25),
            ),
            explanation: "Rewards channel alignment, message focus, and format strength.".into(),
        },
        MetricScore {
            metric: MetricKind::MessageClarity,
            label: metric_label(&MetricKind::MessageClarity),
            score: as_percent(
                (profile.specificity * 0.45)
                    + (profile.tone_conviction * 0.15)
                    + (final_means.get(1).copied().unwrap_or_default() * 0.40),
            ),
            explanation: "Balances specificity, conviction, and simulated resonance.".into(),
        },
        MetricScore {
            metric: MetricKind::ConversionIntent,
            label: metric_label(&MetricKind::ConversionIntent),
            score: as_percent(
                (final_means.get(1).copied().unwrap_or_default() * 0.40)
                    + (profile.preference_fit * 0.25)
                    + ((1.0 - profile.objection_risk) * 0.20)
                    + (profile.relationship_pull * 0.15),
            ),
            explanation: "Estimates likelihood of moving from interest toward action.".into(),
        },
        MetricScore {
            metric: MetricKind::Shareability,
            label: metric_label(&MetricKind::Shareability),
            score: viral_potential,
            explanation: "Derived from simulated share propensity and upside path spread.".into(),
        },
        MetricScore {
            metric: MetricKind::TrustSignal,
            label: metric_label(&MetricKind::TrustSignal),
            score: as_percent(
                ((1.0 - profile.objection_risk) * 0.35)
                    + (profile.relationship_pull * 0.15)
                    + (final_means.get(1).copied().unwrap_or_default() * 0.20)
                    + (profile.specificity * 0.15)
                    + (profile.platform_alignment * 0.15),
            ),
            explanation: "Rewards specificity, lower objection risk, and stronger resonance."
                .into(),
        },
    ];

    apply_scenario_metric_shaping(&mut metrics, scenario_type, approach);
    if let Some(persona) = persona {
        apply_persona_metric_shaping(&mut metrics, persona, approach);
    }

    metrics
}

fn weighted_metric_average(metrics: &[MetricScore], objectives: &[ObjectiveDefinition]) -> u32 {
    if metrics.is_empty() {
        return 0;
    }

    let mut weighted_total = 0.0;
    let mut total_weight = 0.0;
    for objective in objectives {
        if let Some(metric) = metrics
            .iter()
            .find(|metric| metric.metric == objective.metric)
        {
            let weight = objective.weight.max(0.0);
            weighted_total += metric.score as f64 * weight;
            total_weight += weight;
        }
    }

    if total_weight <= f64::EPSILON {
        let average = metrics
            .iter()
            .map(|metric| metric.score as f64)
            .sum::<f64>()
            / metrics.len() as f64;
        average.round() as u32
    } else {
        (weighted_total / total_weight).round() as u32
    }
}

fn blend_persona_scores(
    mut aggregate: PrimaryScorecard,
    persona_results: &[PersonaApproachResult],
    include_metric_breakdown: bool,
) -> PrimaryScorecard {
    if persona_results.is_empty() {
        return aggregate;
    }

    let total_weight = persona_results
        .iter()
        .map(|result| result.audience_weight.max(0.0))
        .sum::<f64>()
        .max(f64::EPSILON);
    let weighted_persona_score = persona_results
        .iter()
        .map(|result| {
            result.primary_scorecard.overall_score as f64 * result.audience_weight.max(0.0)
        })
        .sum::<f64>()
        / total_weight;

    aggregate.overall_score = (((aggregate.overall_score as f64) * 0.55)
        + (weighted_persona_score * 0.45))
        .round() as u32;

    if include_metric_breakdown {
        for metric in &mut aggregate.metrics {
            let matching = persona_results
                .iter()
                .filter_map(|result| {
                    result
                        .primary_scorecard
                        .metrics
                        .iter()
                        .find(|candidate| candidate.metric == metric.metric)
                        .map(|candidate| candidate.score as f64 * result.audience_weight.max(0.0))
                })
                .sum::<f64>()
                / total_weight;
            metric.score = (((metric.score as f64) * 0.55) + (matching * 0.45)).round() as u32;
        }
    }

    aggregate
}

fn summarize_v2_scorecard(
    approach_results: &[ApproachSimulationResultV2],
    include_metric_breakdown: bool,
) -> PrimaryScorecard {
    if approach_results.is_empty() {
        return PrimaryScorecard {
            overall_score: 0,
            metrics: Vec::new(),
        };
    }

    let overall_score = (approach_results
        .iter()
        .map(|result| result.primary_scorecard.overall_score as f64)
        .sum::<f64>()
        / approach_results.len() as f64)
        .round() as u32;

    let metrics = if include_metric_breakdown {
        let metric_kinds = [
            MetricKind::AudienceReceptivity,
            MetricKind::PersonaFit,
            MetricKind::ChannelFit,
            MetricKind::MessageClarity,
            MetricKind::ConversionIntent,
            MetricKind::Shareability,
            MetricKind::TrustSignal,
        ];
        metric_kinds
            .into_iter()
            .filter_map(|kind| {
                let matching = approach_results
                    .iter()
                    .filter_map(|result| {
                        result
                            .primary_scorecard
                            .metrics
                            .iter()
                            .find(|metric| metric.metric == kind)
                            .map(|metric| metric.score as f64)
                    })
                    .collect::<Vec<_>>();
                if matching.is_empty() {
                    None
                } else {
                    Some(MetricScore {
                        metric: kind.clone(),
                        label: metric_label(&kind),
                        score: (matching.iter().sum::<f64>() / matching.len() as f64).round()
                            as u32,
                        explanation: "Average score across simulated approaches in this scenario."
                            .into(),
                    })
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    PrimaryScorecard {
        overall_score,
        metrics,
    }
}

fn metric_label(metric: &MetricKind) -> String {
    match metric {
        MetricKind::AudienceReceptivity => "Audience Receptivity",
        MetricKind::PersonaFit => "Persona Fit",
        MetricKind::ChannelFit => "Channel Fit",
        MetricKind::MessageClarity => "Message Clarity",
        MetricKind::ConversionIntent => "Conversion Intent",
        MetricKind::Shareability => "Shareability",
        MetricKind::TrustSignal => "Trust Signal",
    }
    .into()
}

fn as_percent(value: f64) -> u32 {
    (clamp01(value) * 100.0).round() as u32
}

fn apply_scenario_metric_shaping(
    metrics: &mut [MetricScore],
    scenario_type: &ScenarioType,
    approach: &ApproachDefinition,
) {
    let channels = approach
        .channels
        .iter()
        .flat_map(|value| tokenize(value))
        .collect::<Vec<_>>();

    for metric in metrics {
        let multiplier = match (&metric.metric, scenario_type) {
            (MetricKind::AudienceReceptivity, ScenarioType::AudienceDiscovery) => 1.05,
            (MetricKind::PersonaFit, ScenarioType::AudienceDiscovery) => 1.12,
            (MetricKind::MessageClarity, ScenarioType::Positioning) => 1.15,
            (MetricKind::TrustSignal, ScenarioType::Positioning) => 1.08,
            (MetricKind::ConversionIntent, ScenarioType::CampaignSequence) => 1.18,
            (MetricKind::Shareability, ScenarioType::CampaignSequence) => 1.08,
            (MetricKind::TrustSignal, ScenarioType::CampaignSequence) => 1.06,
            (MetricKind::Shareability, ScenarioType::CommunityActivation) => 1.18,
            (MetricKind::TrustSignal, ScenarioType::CommunityActivation) => 1.10,
            (MetricKind::PersonaFit, ScenarioType::CommunityActivation) => 1.08,
            (MetricKind::ConversionIntent, ScenarioType::CommunityActivation) => 0.92,
            (MetricKind::TrustSignal, ScenarioType::Retention) => 1.18,
            (MetricKind::ConversionIntent, ScenarioType::Retention) => 1.08,
            (MetricKind::Shareability, ScenarioType::Retention) => 0.92,
            _ => 1.0,
        };

        let contextual_bonus = match metric.metric {
            MetricKind::TrustSignal
                if channels.iter().any(|token| {
                    matches!(token.as_str(), "private" | "event" | "dinners" | "community")
                }) =>
            {
                4.0
            }
            MetricKind::Shareability
                if channels.iter().any(|token| {
                    matches!(token.as_str(), "tiktok" | "instagram" | "reels" | "stories")
                }) =>
            {
                4.0
            }
            MetricKind::MessageClarity
                if channels
                    .iter()
                    .any(|token| matches!(token.as_str(), "landing" | "page")) =>
            {
                4.0
            }
            MetricKind::ConversionIntent
                if channels
                    .iter()
                    .any(|token| matches!(token.as_str(), "store" | "in" | "private")) =>
            {
                2.0
            }
            _ => 0.0,
        };

        metric.score = (((metric.score as f64) * multiplier) + contextual_bonus)
            .round()
            .clamp(0.0, 100.0) as u32;
    }
}

fn apply_persona_metric_shaping(
    metrics: &mut [MetricScore],
    persona: &PersonaDefinition,
    approach: &ApproachDefinition,
) {
    let subject_tokens = tokenize(&format!(
        "{} {} {} {} {}",
        approach.angle,
        approach.format,
        approach.tone,
        approach.target,
        approach.channels.join(" ")
    ));
    let channel_tokens = tokenize(&approach.channels.join(" "));
    let jobs_overlap = overlap_score(
        &persona
            .jobs
            .iter()
            .flat_map(|value| tokenize(value))
            .collect::<Vec<_>>(),
        &subject_tokens,
    );
    let trust_overlap = overlap_score(
        &persona
            .trust_signals
            .iter()
            .flat_map(|value| tokenize(value))
            .collect::<Vec<_>>(),
        &subject_tokens,
    );
    let channel_overlap = overlap_score(
        &persona
            .channels
            .iter()
            .flat_map(|value| tokenize(value))
            .collect::<Vec<_>>(),
        &channel_tokens,
    );
    let barrier_overlap = overlap_score(
        &persona
            .conversion_barriers
            .iter()
            .flat_map(|value| tokenize(value))
            .collect::<Vec<_>>(),
        &subject_tokens,
    );
    let proof_score = subject_contains_any(
        &subject_tokens,
        &[
            "proof",
            "data",
            "results",
            "confidence",
            "correlation",
            "review",
            "case",
            "metrics",
        ],
    );
    let privacy_score =
        subject_contains_any(&subject_tokens, &["privacy", "private", "export", "anonymous"]);
    let proof_threshold = persona.proof_threshold.unwrap_or(0.45).clamp(0.0, 1.0);
    let privacy_sensitivity = persona.privacy_sensitivity.unwrap_or(0.20).clamp(0.0, 1.0);
    let price_sensitivity = persona.price_sensitivity.unwrap_or(0.25).clamp(0.0, 1.0);

    for metric in metrics {
        let delta = match metric.metric {
            MetricKind::PersonaFit => {
                (jobs_overlap * 14.0) + (trust_overlap * 10.0) + (channel_overlap * 8.0)
            }
            MetricKind::ChannelFit => channel_overlap * 18.0,
            MetricKind::TrustSignal => {
                (trust_overlap * 12.0)
                    + (proof_score * proof_threshold * 10.0)
                    + (privacy_score * privacy_sensitivity * 10.0)
                    - (barrier_overlap * 8.0)
            }
            MetricKind::ConversionIntent => {
                (jobs_overlap * 8.0)
                    + (proof_score * proof_threshold * 8.0)
                    + (privacy_score * privacy_sensitivity * 6.0)
                    - (barrier_overlap * 10.0)
                    - (price_sensitivity * subject_contains_any(&subject_tokens, &["premium", "expensive"]) * 8.0)
            }
            MetricKind::MessageClarity => {
                (jobs_overlap * 6.0) + (trust_overlap * 4.0) - (barrier_overlap * 4.0)
            }
            MetricKind::Shareability => (channel_overlap * 10.0) + (jobs_overlap * 4.0),
            MetricKind::AudienceReceptivity => {
                (jobs_overlap * 8.0)
                    + (channel_overlap * 6.0)
                    + (proof_score * proof_threshold * 6.0)
                    - (barrier_overlap * 6.0)
            }
        };

        metric.score = ((metric.score as f64) + delta)
            .round()
            .clamp(0.0, 100.0) as u32;
    }
}

fn subject_contains_any(tokens: &[String], keywords: &[&str]) -> f64 {
    if keywords.iter().any(|keyword| tokens.iter().any(|token| token == keyword)) {
        1.0
    } else {
        0.0
    }
}

fn build_scenario(
    index: usize,
    approach: &ApproachInput,
    profile: &AudienceProfile,
    time_steps: usize,
    scenario_type: &ScenarioType,
    dynamics: &ScenarioDynamics,
) -> Scenario {
    let mut scenario = Scenario::new(
        format!("marketing-{}", index + 1),
        format!("Marketing Approach {}", approach.id),
        build_initial_state(profile, dynamics),
        time_steps,
    );
    scenario.failure_threshold = Some(0.45);
    scenario.metadata = Some(serde_json::json!({
        "domain": "marketing",
        "dimension_labels": ["attention", "resonance", "share_propensity"],
        "approach_id": approach.id,
        "scenario_type": scenario_type,
    }));
    scenario.actions = build_actions(profile, time_steps, scenario_type);
    scenario
}

fn build_initial_state(profile: &AudienceProfile, dynamics: &ScenarioDynamics) -> SimState {
    let attention = clamp01(
        0.42 + (profile.format_strength * 0.14)
            + (profile.novelty * 0.14)
            + (profile.platform_alignment * 0.08)
            + (profile.channel_focus * 0.06)
            - (profile.competitor_pressure * 0.10)
            + dynamics.initial_state_bias[0],
    );
    let resonance = clamp01(
        0.44 + (profile.preference_fit * 0.20)
            + (profile.specificity * 0.12)
            + (profile.relationship_pull * 0.06)
            - (profile.objection_risk * 0.18)
            + dynamics.initial_state_bias[1],
    );
    let share_propensity = clamp01(
        0.38 + (profile.tone_conviction * 0.12)
            + (profile.novelty * 0.14)
            + (profile.platform_alignment * 0.10)
            + (profile.persona_focus * 0.08)
            - ((1.0 - profile.channel_focus) * 0.08)
            + dynamics.initial_state_bias[2],
    );
    let fatigue_memory = clamp01(
        0.10 + (profile.competitor_pressure * 0.10)
            + ((1.0 - profile.channel_focus) * 0.12)
            + (profile.objection_risk * 0.08),
    );
    let uncertainty = clamp01(
        0.14 + (profile.competitor_pressure * 0.10)
            + ((1.0 - profile.platform_alignment) * 0.08)
            + ((1.0 - profile.specificity) * 0.06),
    );

    SimState::new(
        vec![attention, resonance, share_propensity],
        vec![
            clamp01(fatigue_memory + dynamics.initial_memory_shift[0]),
            clamp01((fatigue_memory * 0.9) + dynamics.initial_memory_shift[1]),
            clamp01((fatigue_memory * 0.8) + dynamics.initial_memory_shift[2]),
        ],
        vec![
            clamp01(uncertainty + dynamics.initial_uncertainty_shift[0]),
            clamp01((uncertainty * 0.95) + dynamics.initial_uncertainty_shift[1]),
            clamp01((uncertainty * 1.05) + dynamics.initial_uncertainty_shift[2]),
        ],
    )
}

fn build_actions(
    profile: &AudienceProfile,
    time_steps: usize,
    scenario_type: &ScenarioType,
) -> Vec<Action> {
    let skepticism = -(0.08
        + (profile.objection_risk * 0.16)
        + ((1.0 - profile.channel_focus) * 0.10));
    let base_attention = 0.16 + (profile.novelty * 0.16);
    let base_resonance = 0.14 + (profile.preference_fit * 0.14) + (profile.specificity * 0.10);
    let base_share =
        0.12 + (profile.platform_alignment * 0.10) + (profile.tone_conviction * 0.10);
    let recovery = 0.08 + (profile.relationship_pull * 0.10) + (profile.platform_alignment * 0.06);

    let mut actions = match scenario_type {
        ScenarioType::AudienceDiscovery | ScenarioType::Custom => vec![
            action(Some(0), base_attention, ActionType::Intervention, "hook launch"),
            action(
                Some(1),
                base_resonance,
                ActionType::Intervention,
                "message resonance",
            ),
            action(Some(2), base_share, ActionType::Intervention, "sharing catalyst"),
            Action::default(),
            action(Some(1), skepticism, ActionType::StressorOnset, "skepticism"),
            action(
                Some(2),
                recovery,
                ActionType::StressorRemoval,
                "social proof recovery",
            ),
        ],
        ScenarioType::Positioning => vec![
            action(
                Some(1),
                base_resonance * 1.18,
                ActionType::Intervention,
                "headline framing",
            ),
            action(
                Some(0),
                base_attention * 0.88,
                ActionType::Intervention,
                "clarity hook",
            ),
            action(
                Some(1),
                0.10 + (profile.specificity * 0.12),
                ActionType::StressorRemoval,
                "proof clarification",
            ),
            Action::default(),
            action(Some(1), skepticism * 0.82, ActionType::StressorOnset, "skepticism"),
            action(
                Some(1),
                recovery * 0.85,
                ActionType::StressorRemoval,
                "trust recovery",
            ),
        ],
        ScenarioType::CampaignSequence => vec![
            action(Some(0), base_attention * 1.05, ActionType::Intervention, "hook launch"),
            action(
                Some(1),
                base_resonance * 1.08,
                ActionType::Intervention,
                "proof follow-up",
            ),
            action(
                Some(1),
                0.11 + (profile.preference_fit * 0.10),
                ActionType::StressorRemoval,
                "objection handling",
            ),
            action(
                Some(0),
                0.09 + (profile.channel_focus * 0.08),
                ActionType::Intervention,
                "conversion CTA",
            ),
            action(Some(1), skepticism * 0.76, ActionType::StressorOnset, "friction"),
            action(
                Some(2),
                recovery * 0.95,
                ActionType::StressorRemoval,
                "social proof reinforcement",
            ),
        ],
        ScenarioType::CommunityActivation => vec![
            action(
                Some(0),
                base_attention * 0.96,
                ActionType::Intervention,
                "community invitation",
            ),
            action(
                Some(2),
                base_share * 1.25,
                ActionType::Intervention,
                "participation spark",
            ),
            action(
                Some(1),
                base_resonance * 0.94,
                ActionType::Intervention,
                "belonging story",
            ),
            action(
                Some(2),
                0.10 + (profile.relationship_pull * 0.10),
                ActionType::StressorRemoval,
                "peer amplification",
            ),
            action(Some(1), skepticism * 0.90, ActionType::StressorOnset, "skepticism"),
            action(
                Some(2),
                recovery * 1.08,
                ActionType::StressorRemoval,
                "community proof recovery",
            ),
        ],
        ScenarioType::Retention => vec![
            action(
                Some(1),
                base_resonance * 1.12,
                ActionType::Intervention,
                "value reminder",
            ),
            action(
                Some(0),
                base_attention * 0.72,
                ActionType::Intervention,
                "gentle reminder",
            ),
            action(
                Some(1),
                0.10 + (profile.relationship_pull * 0.12),
                ActionType::StressorRemoval,
                "habit reinforcement",
            ),
            Action::default(),
            action(Some(1), skepticism * 0.68, ActionType::StressorOnset, "friction"),
            action(
                Some(1),
                recovery * 0.92,
                ActionType::StressorRemoval,
                "trust restoration",
            ),
        ],
    };

    actions.resize_with(time_steps, Action::default);
    actions.truncate(time_steps);
    actions
}

fn action(
    dimension: Option<usize>,
    magnitude: f64,
    action_type: ActionType,
    label: &str,
) -> Action {
    Action {
        dimension,
        magnitude,
        action_type,
        metadata: Some(serde_json::json!({ "label": label })),
    }
}

fn scenario_dynamics(scenario_type: &ScenarioType) -> ScenarioDynamics {
    match scenario_type {
        ScenarioType::AudienceDiscovery | ScenarioType::Custom => ScenarioDynamics {
            initial_state_bias: [0.0, 0.0, 0.0],
            initial_memory_shift: [0.0, 0.0, 0.0],
            initial_uncertainty_shift: [0.0, 0.0, 0.0],
            action_effect_multipliers: [1.0, 1.0, 1.0],
            action_decay: [0.010, 0.008, 0.006],
            memory_penalty_multipliers: [1.0, 1.0, 1.0],
            uncertainty_penalty_multipliers: [1.0, 1.0, 1.0],
            memory_decay: [0.86, 0.84, 0.82],
            uncertainty_decay: [0.85, 0.84, 0.83],
            resonance_to_attention: 0.030,
            share_to_attention: 0.020,
            attention_to_resonance: 0.020,
            relationship_to_resonance: 0.010,
            attention_to_share: 0.030,
            resonance_to_share: 0.045,
            health_weights: [0.30, 0.40, 0.30],
        },
        ScenarioType::Positioning => ScenarioDynamics {
            initial_state_bias: [-0.02, 0.06, -0.05],
            initial_memory_shift: [-0.01, -0.02, 0.0],
            initial_uncertainty_shift: [-0.01, -0.03, 0.0],
            action_effect_multipliers: [0.90, 1.20, 0.72],
            action_decay: [0.012, 0.005, 0.008],
            memory_penalty_multipliers: [0.92, 0.80, 1.05],
            uncertainty_penalty_multipliers: [0.92, 0.72, 1.0],
            memory_decay: [0.84, 0.80, 0.82],
            uncertainty_decay: [0.83, 0.78, 0.83],
            resonance_to_attention: 0.038,
            share_to_attention: 0.012,
            attention_to_resonance: 0.030,
            relationship_to_resonance: 0.018,
            attention_to_share: 0.018,
            resonance_to_share: 0.030,
            health_weights: [0.24, 0.54, 0.22],
        },
        ScenarioType::CampaignSequence => ScenarioDynamics {
            initial_state_bias: [0.02, 0.03, 0.02],
            initial_memory_shift: [0.0, -0.01, -0.01],
            initial_uncertainty_shift: [-0.01, -0.02, -0.01],
            action_effect_multipliers: [1.08, 1.12, 1.05],
            action_decay: [0.008, 0.006, 0.005],
            memory_penalty_multipliers: [1.0, 0.88, 0.92],
            uncertainty_penalty_multipliers: [0.92, 0.88, 0.90],
            memory_decay: [0.87, 0.82, 0.80],
            uncertainty_decay: [0.84, 0.80, 0.80],
            resonance_to_attention: 0.032,
            share_to_attention: 0.022,
            attention_to_resonance: 0.022,
            relationship_to_resonance: 0.014,
            attention_to_share: 0.032,
            resonance_to_share: 0.050,
            health_weights: [0.28, 0.42, 0.30],
        },
        ScenarioType::CommunityActivation => ScenarioDynamics {
            initial_state_bias: [0.02, 0.01, 0.08],
            initial_memory_shift: [0.0, 0.0, -0.02],
            initial_uncertainty_shift: [0.0, -0.01, -0.02],
            action_effect_multipliers: [0.98, 0.96, 1.28],
            action_decay: [0.010, 0.009, 0.003],
            memory_penalty_multipliers: [1.0, 0.95, 0.72],
            uncertainty_penalty_multipliers: [1.0, 0.92, 0.78],
            memory_decay: [0.86, 0.84, 0.78],
            uncertainty_decay: [0.85, 0.82, 0.79],
            resonance_to_attention: 0.028,
            share_to_attention: 0.030,
            attention_to_resonance: 0.020,
            relationship_to_resonance: 0.012,
            attention_to_share: 0.036,
            resonance_to_share: 0.060,
            health_weights: [0.26, 0.31, 0.43],
        },
        ScenarioType::Retention => ScenarioDynamics {
            initial_state_bias: [-0.04, 0.07, -0.01],
            initial_memory_shift: [-0.02, -0.03, -0.01],
            initial_uncertainty_shift: [-0.02, -0.03, -0.01],
            action_effect_multipliers: [0.78, 1.16, 0.88],
            action_decay: [0.013, 0.004, 0.006],
            memory_penalty_multipliers: [0.84, 0.72, 0.86],
            uncertainty_penalty_multipliers: [0.84, 0.70, 0.88],
            memory_decay: [0.80, 0.76, 0.79],
            uncertainty_decay: [0.80, 0.76, 0.80],
            resonance_to_attention: 0.034,
            share_to_attention: 0.016,
            attention_to_resonance: 0.024,
            relationship_to_resonance: 0.020,
            attention_to_share: 0.026,
            resonance_to_share: 0.040,
            health_weights: [0.22, 0.56, 0.22],
        },
    }
}

impl Simulator for MarketingSimulator {
    fn step(&self, state: &SimState, action: &Action, rng: &mut dyn rand::RngCore) -> SimState {
        let mut next = state.clone();
        next.t += 1;

        let targeted = |dimension: usize| {
            action
                .dimension
                .map(|value| value == dimension)
                .unwrap_or(true)
        };
        let action_magnitude = normalized_action_magnitude(action);
        let action_abs = action_magnitude.abs();

        let noise = [
            (rng.gen::<f64>() - 0.5) * 0.020,
            (rng.gen::<f64>() - 0.5) * 0.018,
            (rng.gen::<f64>() - 0.5) * 0.022,
        ];

        let attention_effect = if targeted(0) {
            action_magnitude
                * (0.10 + (self.profile.novelty * 0.05))
                * self.dynamics.action_effect_multipliers[0]
        } else {
            0.0
        };
        let resonance_effect = if targeted(1) {
            action_magnitude
                * (0.11 + (self.profile.preference_fit * 0.05))
                * self.dynamics.action_effect_multipliers[1]
        } else {
            0.0
        };
        let share_effect = if targeted(2) {
            action_magnitude
                * (0.12 + (self.profile.platform_alignment * 0.05))
                * self.dynamics.action_effect_multipliers[2]
        } else {
            0.0
        };

        next.z[0] = clamp01(
            state.z[0]
                - self.dynamics.action_decay[0]
                + attention_effect
                + (state.z[1] * self.dynamics.resonance_to_attention)
                + (state.z[2] * self.dynamics.share_to_attention)
                - (state.m[0]
                    * (0.090 + (self.profile.competitor_pressure * 0.050))
                    * self.dynamics.memory_penalty_multipliers[0])
                - (state.u[0] * 0.030 * self.dynamics.uncertainty_penalty_multipliers[0])
                + noise[0],
        );
        next.z[1] = clamp01(
            state.z[1]
                - self.dynamics.action_decay[1]
                + resonance_effect
                + (state.z[0] * self.dynamics.attention_to_resonance)
                + (self.profile.relationship_pull * self.dynamics.relationship_to_resonance)
                - (state.m[1]
                    * (0.085 + (self.profile.objection_risk * 0.070))
                    * self.dynamics.memory_penalty_multipliers[1])
                - (state.u[1] * 0.025 * self.dynamics.uncertainty_penalty_multipliers[1])
                + noise[1],
        );
        next.z[2] = clamp01(
            state.z[2]
                - self.dynamics.action_decay[2]
                + share_effect
                + (next.z[0] * self.dynamics.attention_to_share)
                + (next.z[1] * self.dynamics.resonance_to_share)
                - (state.m[2]
                    * (0.070 + ((1.0 - self.profile.channel_focus) * 0.050))
                    * self.dynamics.memory_penalty_multipliers[2])
                - (state.u[2] * 0.020 * self.dynamics.uncertainty_penalty_multipliers[2])
                + noise[2],
        );

        next.m[0] = clamp01(
            (state.m[0] * self.dynamics.memory_decay[0])
                + (action_abs * 0.10)
                + ((1.0 - self.profile.channel_focus) * 0.020),
        );
        next.m[1] = clamp01(
            (state.m[1] * self.dynamics.memory_decay[1])
                + (action_abs * 0.09)
                + (self.profile.objection_risk * 0.030),
        );
        next.m[2] = clamp01(
            (state.m[2] * self.dynamics.memory_decay[2])
                + (action_abs * 0.08)
                + ((1.0 - self.profile.platform_alignment) * 0.020),
        );

        next.u[0] = clamp01(
            (state.u[0] * self.dynamics.uncertainty_decay[0])
                + (self.profile.competitor_pressure * 0.020),
        );
        next.u[1] = clamp01(
            (state.u[1] * self.dynamics.uncertainty_decay[1])
                + ((1.0 - self.profile.specificity) * 0.020),
        );
        next.u[2] = clamp01(
            (state.u[2] * self.dynamics.uncertainty_decay[2])
                + ((1.0 - self.profile.platform_alignment) * 0.020),
        );

        next
    }

    fn health_index(&self, state: &SimState) -> f64 {
        ((state.z[0] * self.dynamics.health_weights[0])
            + (state.z[1] * self.dynamics.health_weights[1])
            + (state.z[2] * self.dynamics.health_weights[2]))
            .clamp(0.0, 1.0)
    }
}

fn build_audience_profile(
    seed_data: &SeedData,
    approach: &ApproachInput,
    platforms: &[String],
) -> AudienceProfile {
    let subject_tokens = tokenize(&format!(
        "{} {} {} {} {}",
        approach.angle,
        approach.format,
        approach.tone,
        approach.target,
        approach.channels.join(" ")
    ));
    let platform_tokens = tokenize(&platforms.join(" "));
    let project_tokens = tokenize(&format!(
        "{} {}",
        seed_data.project_name, seed_data.project_description
    ));
    let audience_tokens = seed_data
        .personas
        .iter()
        .flat_map(|persona| {
            let mut tokens = Vec::new();
            tokens.extend(tokenize(&persona.name));
            tokens.extend(tokenize(&persona.persona_type));
            if let Some(relationship) = &persona.relationship {
                tokens.extend(tokenize(relationship));
            }
            if let Some(value) = &persona.demographics {
                tokens.extend(json_tokens(value));
            }
            if let Some(value) = &persona.psychographics {
                tokens.extend(json_tokens(value));
            }
            tokens
        })
        .collect::<Vec<_>>();

    let preference_fit = average_or(
        seed_data.personas.iter().map(|persona| {
            overlap_score(
                &persona
                    .preferences
                    .iter()
                    .flat_map(|value| tokenize(value))
                    .collect::<Vec<_>>(),
                &subject_tokens,
            )
        }),
        0.45,
    );
    let objection_risk = average_or(
        seed_data.personas.iter().map(|persona| {
            overlap_score(
                &persona
                    .objections
                    .iter()
                    .flat_map(|value| tokenize(value))
                    .collect::<Vec<_>>(),
                &subject_tokens,
            )
        }),
        0.10,
    );
    let persona_focus = overlap_score(&audience_tokens, &subject_tokens);
    let relationship_pull = average_or(
        seed_data.personas.iter().map(|persona| {
            persona
                .relationship
                .as_deref()
                .map(|value| overlap_score(&tokenize(value), &subject_tokens))
                .unwrap_or(0.30)
        }),
        0.30,
    );
    let competitor_pressure = overlap_score(
        &seed_data
            .competitors
            .iter()
            .flat_map(|value| tokenize(value))
            .collect::<Vec<_>>(),
        &subject_tokens,
    );
    let platform_alignment = {
        let merged_platforms = [
            platform_tokens.clone(),
            tokenize(&seed_data.platform_context.join(" ")),
        ]
        .concat();
        let alignment = overlap_score(&merged_platforms, &tokenize(&approach.channels.join(" ")));
        if alignment > 0.0 {
            alignment
        } else if approach.channels.is_empty() {
            0.30
        } else {
            0.55
        }
    };
    let channel_focus = 1.0 / (approach.channels.len().max(1) as f64).sqrt();
    let format_strength = score_keywords(
        &subject_tokens,
        &[
            ("thread", 0.75),
            ("carousel", 0.74),
            ("video", 0.80),
            ("short", 0.72),
            ("case", 0.66),
            ("story", 0.70),
            ("guide", 0.64),
            ("checklist", 0.62),
            ("template", 0.60),
        ],
        0.45,
    );
    let tone_conviction = score_keywords(
        &subject_tokens,
        &[
            ("direct", 0.78),
            ("contrarian", 0.82),
            ("honest", 0.74),
            ("warm", 0.64),
            ("authoritative", 0.76),
            ("data", 0.70),
            ("playful", 0.60),
        ],
        0.48,
    );
    let novelty = novelty_score(&subject_tokens, &project_tokens);
    let specificity = clamp01(
        0.25 + (if approach.target.trim().is_empty() {
            0.0
        } else {
            0.30
        }) + (if approach.angle.split_whitespace().count() >= 6 {
            0.15
        } else {
            0.05
        }) + (persona_focus * 0.20),
    );

    AudienceProfile {
        preference_fit,
        objection_risk,
        persona_focus,
        relationship_pull,
        competitor_pressure,
        platform_alignment,
        channel_focus,
        format_strength,
        tone_conviction,
        novelty,
        specificity,
    }
}

fn compute_engagement_score(summary: &RunSummary, final_means: &[f64]) -> u32 {
    let monte = summary.monte_carlo.as_ref();
    let end = monte.and_then(|value| value.end).unwrap_or(0.0);
    let auc = monte.and_then(|value| value.auc).unwrap_or(end);
    let auc_norm = if let Some(steps) = monte.map(|value| value.time_steps) {
        if steps == 0 {
            end
        } else {
            auc / steps as f64
        }
    } else {
        end
    };
    let score = clamp01((end * 0.45) + (final_means[1] * 0.35) + (auc_norm * 0.20));
    (score * 100.0).round() as u32
}

fn compute_viral_potential(
    summary: &RunSummary,
    final_means: &[f64],
    profile: &AudienceProfile,
) -> u32 {
    let monte = summary.monte_carlo.as_ref();
    let p90_end = monte.and_then(|value| value.p90_end).unwrap_or(0.0);
    let score = clamp01(
        (final_means[2] * 0.40)
            + (p90_end * 0.22)
            + (profile.novelty * 0.18)
            + (profile.platform_alignment * 0.12)
            + (profile.tone_conviction * 0.08),
    );
    (score * 100.0).round() as u32
}

fn derive_sentiment_distribution(
    final_means: &[f64],
    profile: &AudienceProfile,
    engagement_score: u32,
) -> SentimentDistribution {
    let positive = clamp01(
        (final_means[1] * 0.60)
            + (profile.preference_fit * 0.18)
            + ((engagement_score as f64 / 100.0) * 0.10)
            - (profile.objection_risk * 0.18),
    );
    let negative =
        clamp01(0.08 + (profile.objection_risk * 0.55) + ((1.0 - final_means[1]) * 0.18));
    let neutral = (1.0 - positive - negative).max(0.0);
    let total = positive + negative + neutral;

    let positive_pct = ((positive / total) * 100.0).round() as i32;
    let negative_pct = ((negative / total) * 100.0).round() as i32;
    let mut neutral_pct = 100 - positive_pct - negative_pct;
    if neutral_pct < 0 {
        neutral_pct = 0;
    }

    SentimentDistribution {
        positive: positive_pct as u32,
        neutral: neutral_pct as u32,
        negative: negative_pct as u32,
    }
}

fn build_top_reactions(approach: &ApproachInput, profile: &AudienceProfile) -> Vec<String> {
    let mut candidates = vec![
        (
            profile.preference_fit,
            format!(
                "Feels tuned to people who already care about {}",
                approach.target.trim()
            ),
        ),
        (
            profile.format_strength,
            format!(
                "The {} format gives the idea a clean delivery vehicle",
                approach.format.trim()
            ),
        ),
        (
            profile.tone_conviction,
            format!(
                "The {} tone gives the angle a clearer point of view",
                approach.tone.trim()
            ),
        ),
        (
            profile.platform_alignment,
            format!(
                "The channel mix matches how this audience already talks on {}",
                display_channels(&approach.channels)
            ),
        ),
        (
            profile.specificity,
            "The setup is specific enough to feel intentional instead of generic".into(),
        ),
    ];
    candidates.sort_by(|left, right| {
        right
            .0
            .partial_cmp(&left.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    candidates
        .into_iter()
        .filter(|(score, _)| *score >= 0.32)
        .take(3)
        .map(|(_, message)| sanitize_phrase(message))
        .collect()
}

fn build_concerns(approach: &ApproachInput, profile: &AudienceProfile) -> Vec<String> {
    let mut concerns = vec![
        (
            profile.objection_risk,
            "Some of the language may trigger skeptical or hype-averse audience members".into(),
        ),
        (
            1.0 - profile.channel_focus,
            format!(
                "Spreading one concept across {} can dilute the core hook",
                display_channels(&approach.channels)
            ),
        ),
        (
            profile.competitor_pressure,
            "The framing risks blending into competitor narratives already in market".into(),
        ),
        (
            1.0 - profile.platform_alignment,
            "The concept may need platform-specific shaping before it feels native".into(),
        ),
        (
            1.0 - profile.specificity,
            "The target segment is still broad enough that response could flatten".into(),
        ),
    ];
    concerns.sort_by(|left, right| {
        right
            .0
            .partial_cmp(&left.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    concerns
        .into_iter()
        .filter(|(score, _)| *score >= 0.28)
        .take(3)
        .map(|(_, message)| sanitize_phrase(message))
        .collect()
}

fn build_emergent_behaviors(
    approach: &ApproachInput,
    archetype: Archetype,
    engagement_score: u32,
    viral_potential: u32,
    profile: &AudienceProfile,
) -> Vec<String> {
    let mut behaviors = Vec::new();

    if engagement_score >= 70 {
        behaviors.push("Saves for later and comes back with higher intent".into());
    }
    if viral_potential >= 72 {
        behaviors.push("Gets forwarded in private chats before it spreads publicly".into());
    }
    if profile.preference_fit >= 0.55 {
        behaviors.push(format!(
            "Core audience starts quoting the {} angle back in comments",
            approach.tone.trim()
        ));
    }
    if profile.objection_risk >= 0.35 {
        behaviors.push("Early replies debate the framing before sentiment settles".into());
    }

    match archetype {
        Archetype::Phoenix => {
            behaviors.push("Initial hesitation flips once the value proposition lands".into())
        }
        Archetype::Oscillator => behaviors
            .push("Response comes in waves as different sub-audiences take turns engaging".into()),
        Archetype::CliffFaller => behaviors
            .push("Attention spikes quickly but fades if there is no follow-up sequence".into()),
        Archetype::Surge => behaviors
            .push("Momentum compounds as repeated exposure increases perceived relevance".into()),
        _ => {}
    }

    behaviors.truncate(4);
    behaviors
}

fn build_cross_approach_insights(
    approaches: &[ApproachInput],
    results: &[ApproachSimulationResult],
) -> Vec<String> {
    if results.is_empty() {
        return Vec::new();
    }

    let mut insights = Vec::new();
    if let Some(best_engagement) = results.iter().max_by_key(|result| result.engagement_score) {
        insights.push(format!(
            "{} has the strongest predicted engagement ceiling",
            approach_label(approaches, &best_engagement.approach_id)
        ));
    }
    if let Some(best_viral) = results.iter().max_by_key(|result| result.viral_potential) {
        insights.push(format!(
            "{} has the best private-share to public-spread profile",
            approach_label(approaches, &best_viral.approach_id)
        ));
    }

    let shared_concern = results
        .iter()
        .flat_map(|result| result.concerns.iter())
        .find(|concern| {
            results
                .iter()
                .filter(|result| result.concerns.contains(*concern))
                .count()
                > 1
        });
    if let Some(concern) = shared_concern {
        insights.push(format!(
            "Multiple approaches share the same constraint: {}",
            concern
        ));
    }

    if results.len() >= 2 {
        let mut sorted = results.iter().collect::<Vec<_>>();
        sorted.sort_by(|left, right| right.engagement_score.cmp(&left.engagement_score));
        let gap = sorted[0]
            .engagement_score
            .saturating_sub(sorted[1].engagement_score);
        if gap >= 8 {
            insights.push(format!(
                "{} opens a clear lead over the next-best option by {} points",
                approach_label(approaches, &sorted[0].approach_id),
                gap
            ));
        }
    }

    insights
}

fn build_cross_approach_insights_v2(
    approaches: &[ApproachDefinition],
    results: &[ApproachSimulationResultV2],
) -> Vec<String> {
    if results.is_empty() {
        return Vec::new();
    }

    let mut insights = Vec::new();
    if let Some(best_overall) = results
        .iter()
        .max_by_key(|result| result.primary_scorecard.overall_score)
    {
        insights.push(format!(
            "{} has the strongest overall fit in this scenario",
            approach_label_v2(approaches, &best_overall.approach_id)
        ));
    }
    if let Some(best_viral) = results.iter().max_by_key(|result| result.viral_potential) {
        insights.push(format!(
            "{} has the best private-share to public-spread profile",
            approach_label_v2(approaches, &best_viral.approach_id)
        ));
    }

    let shared_concern = results
        .iter()
        .flat_map(|result| result.concerns.iter())
        .find(|concern| {
            results
                .iter()
                .filter(|result| result.concerns.contains(*concern))
                .count()
                > 1
        });
    if let Some(concern) = shared_concern {
        insights.push(format!(
            "Multiple approaches share the same constraint: {}",
            concern
        ));
    }

    if results.len() >= 2 {
        let mut sorted = results.iter().collect::<Vec<_>>();
        sorted.sort_by(|left, right| {
            right
                .primary_scorecard
                .overall_score
                .cmp(&left.primary_scorecard.overall_score)
        });
        let gap = sorted[0]
            .primary_scorecard
            .overall_score
            .saturating_sub(sorted[1].primary_scorecard.overall_score);
        if gap >= 8 {
            insights.push(format!(
                "{} opens a clear lead over the next-best option by {} points",
                approach_label_v2(approaches, &sorted[0].approach_id),
                gap
            ));
        }
    }

    insights
}

fn approach_label(approaches: &[ApproachInput], approach_id: &str) -> String {
    approaches
        .iter()
        .find(|approach| approach.id == approach_id)
        .map(|approach| approach.angle.clone())
        .unwrap_or_else(|| approach_id.to_string())
}

fn approach_label_v2(approaches: &[ApproachDefinition], approach_id: &str) -> String {
    approaches
        .iter()
        .find(|approach| approach.id == approach_id)
        .map(|approach| approach.angle.clone())
        .unwrap_or_else(|| approach_id.to_string())
}

fn mean_final_state(result: &composure_core::MonteCarloResult) -> Vec<f64> {
    if result.paths.is_empty() {
        return vec![0.0, 0.0, 0.0];
    }

    let dimensions = result.paths[0].final_state.z.len();
    let mut totals = vec![0.0; dimensions];

    for path in &result.paths {
        for (index, value) in path.final_state.z.iter().enumerate() {
            totals[index] += value;
        }
    }

    totals
        .into_iter()
        .map(|value| value / result.paths.len() as f64)
        .collect()
}

fn display_channels(channels: &[String]) -> String {
    if channels.is_empty() {
        "owned channels".into()
    } else {
        channels.join(", ")
    }
}

fn sanitize_phrase(value: String) -> String {
    value.replace("  ", " ")
}

fn derive_seed_base(request: &MarketingSimulationRequest, simulation_id: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    simulation_id.hash(&mut hasher);
    request.simulation_size.hash(&mut hasher);
    hasher.finish()
}

fn derive_seed_base_v2(request: &MarketingSimulationRequestV2, simulation_id: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    simulation_id.hash(&mut hasher);
    request.simulation_size.hash(&mut hasher);
    request.scenario.time_steps.hash(&mut hasher);
    hasher.finish()
}

fn build_simulation_id(request: &MarketingSimulationRequest) -> String {
    let mut hasher = DefaultHasher::new();
    request.seed_data.project_name.hash(&mut hasher);
    request.seed_data.project_description.hash(&mut hasher);
    request.simulation_size.hash(&mut hasher);
    for approach in &request.approaches {
        approach.id.hash(&mut hasher);
        approach.angle.hash(&mut hasher);
        approach.format.hash(&mut hasher);
        approach.tone.hash(&mut hasher);
        approach.target.hash(&mut hasher);
        for channel in &approach.channels {
            channel.hash(&mut hasher);
        }
    }
    format!("sim-{:016x}", hasher.finish())
}

fn build_simulation_id_v2(request: &MarketingSimulationRequestV2) -> String {
    let mut hasher = DefaultHasher::new();
    serde_json::to_string(request)
        .unwrap_or_default()
        .hash(&mut hasher);
    format!("sim-{:016x}", hasher.finish())
}

fn normalized_action_magnitude(action: &Action) -> f64 {
    match action.action_type {
        ActionType::Hold => 0.0,
        ActionType::StressorRemoval => action.magnitude.abs(),
        _ => action.magnitude,
    }
}

fn score_keywords(tokens: &[String], weights: &[(&str, f64)], fallback: f64) -> f64 {
    let mut score = fallback;
    for (keyword, weight) in weights {
        if tokens.iter().any(|token| token == keyword) {
            score = score.max(*weight);
        }
    }
    clamp01(score)
}

fn novelty_score(subject_tokens: &[String], project_tokens: &[String]) -> f64 {
    let overlap = overlap_score(project_tokens, subject_tokens);
    clamp01(0.55 + ((1.0 - overlap) * 0.30))
}

fn overlap_score(left: &[String], right: &[String]) -> f64 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    let overlap = left.iter().filter(|token| right.contains(*token)).count() as f64;
    clamp01(overlap / left.len().max(right.len()) as f64 * 2.0)
}

fn average_or(values: impl Iterator<Item = f64>, fallback: f64) -> f64 {
    let collected = values.collect::<Vec<_>>();
    if collected.is_empty() {
        fallback
    } else {
        clamp01(collected.iter().sum::<f64>() / collected.len() as f64)
    }
}

fn tokenize(value: &str) -> Vec<String> {
    value
        .to_lowercase()
        .split(|char: char| !char.is_ascii_alphanumeric())
        .filter(|token| token.len() >= 3)
        .map(str::to_string)
        .collect()
}

fn json_tokens(value: &Value) -> Vec<String> {
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) => Vec::new(),
        Value::String(string) => tokenize(string),
        Value::Array(values) => values.iter().flat_map(json_tokens).collect(),
        Value::Object(map) => map
            .iter()
            .flat_map(|(key, value)| {
                let mut tokens = tokenize(key);
                tokens.extend(json_tokens(value));
                tokens
            })
            .collect(),
    }
}

fn default_simulation_size() -> usize {
    64
}

fn default_time_steps() -> usize {
    6
}

fn default_true() -> bool {
    true
}

fn default_objective_weight() -> f64 {
    1.0
}

fn default_relative_weight() -> f64 {
    1.0
}

fn clamp01(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_request() -> MarketingSimulationRequest {
        MarketingSimulationRequest {
            seed_data: SeedData {
                personas: vec![
                    PersonaSeed {
                        name: "Pragmatic Dev".into(),
                        persona_type: "developer".into(),
                        demographics: None,
                        psychographics: Some(serde_json::json!({
                            "goals": ["ship faster", "learn practical systems"]
                        })),
                        relationship: Some("existing customer".into()),
                        preferences: vec!["practical examples".into(), "clear frameworks".into()],
                        objections: vec!["vague marketing".into(), "tool sprawl".into()],
                    },
                    PersonaSeed {
                        name: "Skeptical Founder".into(),
                        persona_type: "founder".into(),
                        demographics: None,
                        psychographics: Some(serde_json::json!({
                            "goals": ["distribution", "credibility"]
                        })),
                        relationship: Some("aware but unconvinced".into()),
                        preferences: vec!["proof".into(), "positioning".into()],
                        objections: vec!["hype".into(), "too broad".into()],
                    },
                ],
                competitors: vec!["generic ai tools".into()],
                project_name: "Composure".into(),
                project_description: "Deterministic simulation for campaigns".into(),
                platform_context: vec!["twitter".into(), "linkedin".into()],
            },
            approaches: vec![
                ApproachInput {
                    id: "specific".into(),
                    angle: "Show founders how to rank hooks before publishing".into(),
                    format: "Twitter thread".into(),
                    channels: vec!["twitter".into()],
                    tone: "direct and contrarian".into(),
                    target: "technical founders".into(),
                },
                ApproachInput {
                    id: "broad".into(),
                    angle: "AI can help everyone do marketing better".into(),
                    format: "blog post".into(),
                    channels: vec!["twitter".into(), "linkedin".into(), "reddit".into()],
                    tone: "friendly".into(),
                    target: "everyone".into(),
                },
            ],
            simulation_size: 24,
            platforms: vec!["twitter".into(), "linkedin".into()],
        }
    }

    fn sample_request_v2() -> MarketingSimulationRequestV2 {
        MarketingSimulationRequestV2 {
            project: ProjectContext {
                name: "Composure".into(),
                description: "Deterministic simulation for campaigns".into(),
                category: Some("developer tools".into()),
                competitors: vec!["generic ai tools".into()],
                platform_context: vec!["twitter".into(), "linkedin".into()],
                constraints: vec!["no hype".into()],
                tags: vec!["simulation".into()],
            },
            personas: vec![
                PersonaDefinition {
                    id: "dev".into(),
                    name: "Pragmatic Dev".into(),
                    persona_type: "developer".into(),
                    demographics: None,
                    psychographics: Some(serde_json::json!({
                        "goals": ["ship faster", "learn practical systems"]
                    })),
                    relationship: Some("existing customer".into()),
                    jobs: vec!["ship faster".into(), "learn practical systems".into()],
                    preferences: vec!["practical examples".into(), "clear frameworks".into()],
                    objections: vec!["vague marketing".into(), "tool sprawl".into()],
                    channels: vec!["twitter".into()],
                    conversion_barriers: vec!["vague marketing".into()],
                    trust_signals: vec!["proof".into(), "examples".into()],
                    price_sensitivity: Some(0.20),
                    proof_threshold: Some(0.35),
                    privacy_sensitivity: Some(0.10),
                },
                PersonaDefinition {
                    id: "founder".into(),
                    name: "Skeptical Founder".into(),
                    persona_type: "founder".into(),
                    demographics: None,
                    psychographics: Some(serde_json::json!({
                        "goals": ["distribution", "credibility"]
                    })),
                    relationship: Some("aware but unconvinced".into()),
                    jobs: vec!["distribution".into(), "credibility".into()],
                    preferences: vec!["proof".into(), "positioning".into()],
                    objections: vec!["hype".into(), "too broad".into()],
                    channels: vec!["linkedin".into(), "twitter".into()],
                    conversion_barriers: vec!["too broad".into(), "hype".into()],
                    trust_signals: vec!["proof".into(), "credible".into()],
                    price_sensitivity: Some(0.35),
                    proof_threshold: Some(0.75),
                    privacy_sensitivity: Some(0.15),
                },
            ],
            approaches: vec![
                ApproachDefinition {
                    id: "specific".into(),
                    angle: "Show founders how to rank hooks before publishing".into(),
                    format: "Twitter thread".into(),
                    channels: vec!["twitter".into()],
                    tone: "direct and contrarian".into(),
                    target: "technical founders".into(),
                    objectives: vec![],
                },
                ApproachDefinition {
                    id: "broad".into(),
                    angle: "AI can help everyone do marketing better".into(),
                    format: "blog post".into(),
                    channels: vec!["twitter".into(), "linkedin".into(), "reddit".into()],
                    tone: "friendly".into(),
                    target: "everyone".into(),
                    objectives: vec![],
                },
            ],
            channels: vec![
                ChannelContext {
                    channel: "twitter".into(),
                    norms: vec!["direct".into(), "threads".into()],
                    constraints: vec!["fast feedback".into()],
                    relative_weight: 1.0,
                },
                ChannelContext {
                    channel: "linkedin".into(),
                    norms: vec!["credibility".into()],
                    constraints: vec![],
                    relative_weight: 0.8,
                },
            ],
            audience_weighting: vec![
                AudienceWeighting {
                    persona_id: "dev".into(),
                    weight: 0.9,
                },
                AudienceWeighting {
                    persona_id: "founder".into(),
                    weight: 1.1,
                },
            ],
            scenario: ScenarioDefinition {
                name: "ICP discovery".into(),
                description: Some("Minimal V2 scaffold validation".into()),
                scenario_type: ScenarioType::AudienceDiscovery,
                time_steps: 6,
                objectives: vec![],
            },
            output: OutputOptions::default(),
            simulation_size: 24,
        }
    }

    #[test]
    fn marketing_simulation_is_deterministic() {
        let request = sample_request();
        let first = simulate_marketing(&request).unwrap();
        let second = simulate_marketing(&request).unwrap();

        assert_eq!(first.simulation_id, second.simulation_id);
        assert_eq!(
            serde_json::to_string(&first).unwrap(),
            serde_json::to_string(&second).unwrap()
        );
    }

    #[test]
    fn targeted_approach_scores_better_than_broad_generic_one() {
        let request = sample_request();
        let result = simulate_marketing(&request).unwrap();

        let specific = result
            .approach_results
            .iter()
            .find(|value| value.approach_id == "specific")
            .unwrap();
        let broad = result
            .approach_results
            .iter()
            .find(|value| value.approach_id == "broad")
            .unwrap();

        assert!(specific.engagement_score > broad.engagement_score);
        assert!(specific.viral_potential >= broad.viral_potential);
    }

    #[test]
    fn marketing_simulation_v2_returns_persona_breakdowns_and_metrics() {
        let request = sample_request_v2();
        let result = simulate_marketing_v2(&request).unwrap();

        assert_eq!(result.approach_results.len(), 2);
        assert_eq!(result.scenario.name, "ICP discovery");
        assert!(!result.primary_scorecard.metrics.is_empty());

        let specific = result
            .approach_results
            .iter()
            .find(|value| value.approach_id == "specific")
            .unwrap();
        let broad = result
            .approach_results
            .iter()
            .find(|value| value.approach_id == "broad")
            .unwrap();

        assert_eq!(specific.persona_results.len(), 2);
        assert!(specific.primary_scorecard.overall_score > broad.primary_scorecard.overall_score);
        assert!(specific
            .primary_scorecard
            .metrics
            .iter()
            .any(|metric| metric.metric == MetricKind::AudienceReceptivity));
    }

    #[test]
    fn marketing_simulation_v2_can_omit_breakdowns_and_trajectories() {
        let mut request = sample_request_v2();
        request.output.include_metric_breakdown = false;
        request.output.include_persona_breakdown = false;
        request.output.include_mean_trajectory = false;

        let result = simulate_marketing_v2(&request).unwrap();
        let first = result.approach_results.first().unwrap();

        assert!(result.primary_scorecard.metrics.is_empty());
        assert!(first.persona_results.is_empty());
        assert!(first.mean_trajectory.is_empty());
    }

    #[test]
    fn marketing_simulation_v2_preserves_persona_scores_without_metric_breakdowns() {
        let mut request = MarketingSimulationRequestV2 {
            project: ProjectContext {
                name: "Composure".into(),
                description: "Deterministic simulation for campaigns".into(),
                category: None,
                competitors: vec![],
                platform_context: vec!["landing page".into(), "reddit".into()],
                constraints: vec![],
                tags: vec![],
            },
            personas: vec![
                PersonaDefinition {
                    id: "privacy".into(),
                    name: "Privacy-Sensitive Notes User".into(),
                    persona_type: "privacy_user".into(),
                    demographics: None,
                    psychographics: None,
                    relationship: Some("aware but unconvinced".into()),
                    jobs: vec!["keep data private".into(), "replace scattered notes".into()],
                    preferences: vec!["privacy".into(), "export".into()],
                    objections: vec!["too much setup".into()],
                    channels: vec!["reddit".into()],
                    conversion_barriers: vec!["health data privacy".into()],
                    trust_signals: vec!["private".into(), "export".into()],
                    price_sensitivity: None,
                    proof_threshold: Some(0.20),
                    privacy_sensitivity: Some(0.95),
                },
                PersonaDefinition {
                    id: "proof".into(),
                    name: "Proof-Hungry Optimizer".into(),
                    persona_type: "optimizer".into(),
                    demographics: None,
                    psychographics: None,
                    relationship: Some("solution aware".into()),
                    jobs: vec!["see correlations".into(), "measure deltas".into()],
                    preferences: vec!["proof".into(), "confidence".into()],
                    objections: vec!["vague claims".into()],
                    channels: vec!["landing page".into()],
                    conversion_barriers: vec!["not enough data".into()],
                    trust_signals: vec!["proof".into(), "confidence".into(), "correlation".into()],
                    price_sensitivity: None,
                    proof_threshold: Some(0.95),
                    privacy_sensitivity: Some(0.05),
                },
            ],
            approaches: vec![ApproachDefinition {
                id: "privacy-proof".into(),
                angle: "A private way to track protocols with proof confidence and exportable reports".into(),
                format: "landing page headline".into(),
                channels: vec!["landing page".into(), "reddit".into()],
                tone: "calm and credible".into(),
                target: "people who want proof and privacy".into(),
                objectives: vec![],
            }],
            channels: vec![],
            audience_weighting: vec![],
            scenario: ScenarioDefinition {
                name: "positioning".into(),
                description: None,
                scenario_type: ScenarioType::Positioning,
                time_steps: 6,
                objectives: vec![],
            },
            output: OutputOptions::default(),
            simulation_size: 24,
        };
        request.output.include_metric_breakdown = false;

        let result = simulate_marketing_v2(&request).unwrap();
        let first = result.approach_results.first().unwrap();

        assert!(!first.persona_results.is_empty());
        assert!(first
            .persona_results
            .iter()
            .all(|persona| persona.primary_scorecard.metrics.is_empty()));
        assert!(first
            .persona_results
            .iter()
            .all(|persona| persona.engagement_score > 0 || persona.viral_potential > 0));
        assert!(first
            .persona_results
            .windows(2)
            .any(|window| window[0].engagement_score != window[1].engagement_score
                || window[0].viral_potential != window[1].viral_potential));
    }

    #[test]
    fn marketing_simulation_v2_persona_scores_diverge_when_persona_traits_diverge() {
        let request = MarketingSimulationRequestV2 {
            project: ProjectContext {
                name: "Composure".into(),
                description: "Deterministic simulation for campaigns".into(),
                category: None,
                competitors: vec![],
                platform_context: vec!["landing page".into(), "reddit".into()],
                constraints: vec![],
                tags: vec![],
            },
            personas: vec![
                PersonaDefinition {
                    id: "privacy".into(),
                    name: "Privacy-Sensitive Notes User".into(),
                    persona_type: "privacy_user".into(),
                    demographics: None,
                    psychographics: None,
                    relationship: Some("aware but unconvinced".into()),
                    jobs: vec!["keep data private".into(), "replace scattered notes".into()],
                    preferences: vec!["privacy".into(), "export".into()],
                    objections: vec!["too much setup".into()],
                    channels: vec!["reddit".into()],
                    conversion_barriers: vec!["health data privacy".into()],
                    trust_signals: vec!["private".into(), "export".into()],
                    price_sensitivity: None,
                    proof_threshold: Some(0.20),
                    privacy_sensitivity: Some(0.95),
                },
                PersonaDefinition {
                    id: "proof".into(),
                    name: "Proof-Hungry Optimizer".into(),
                    persona_type: "optimizer".into(),
                    demographics: None,
                    psychographics: None,
                    relationship: Some("solution aware".into()),
                    jobs: vec!["see correlations".into(), "measure deltas".into()],
                    preferences: vec!["proof".into(), "confidence".into()],
                    objections: vec!["vague claims".into()],
                    channels: vec!["landing page".into()],
                    conversion_barriers: vec!["not enough data".into()],
                    trust_signals: vec!["proof".into(), "confidence".into(), "correlation".into()],
                    price_sensitivity: None,
                    proof_threshold: Some(0.95),
                    privacy_sensitivity: Some(0.05),
                },
            ],
            approaches: vec![ApproachDefinition {
                id: "privacy-proof".into(),
                angle: "A private way to track protocols with proof confidence and exportable reports".into(),
                format: "landing page headline".into(),
                channels: vec!["landing page".into(), "reddit".into()],
                tone: "calm and credible".into(),
                target: "people who want proof and privacy".into(),
                objectives: vec![],
            }],
            channels: vec![],
            audience_weighting: vec![],
            scenario: ScenarioDefinition {
                name: "positioning".into(),
                description: None,
                scenario_type: ScenarioType::Positioning,
                time_steps: 6,
                objectives: vec![],
            },
            output: OutputOptions::default(),
            simulation_size: 24,
        };
        let result = simulate_marketing_v2(&request).unwrap();
        let specific = result
            .approach_results
            .iter()
            .find(|value| value.approach_id == "privacy-proof")
            .unwrap();

        let privacy = specific
            .persona_results
            .iter()
            .find(|value| value.persona_id == "privacy")
            .unwrap();
        let proof = specific
            .persona_results
            .iter()
            .find(|value| value.persona_id == "proof")
            .unwrap();

        assert!(
            privacy
                .primary_scorecard
                .metrics
                .iter()
                .zip(proof.primary_scorecard.metrics.iter())
                .any(|(left, right)| left.score != right.score)
        );
        assert_ne!(
            privacy
                .primary_scorecard
                .metrics
                .iter()
                .find(|metric| metric.metric == MetricKind::TrustSignal)
                .unwrap()
                .score,
            proof
                .primary_scorecard
                .metrics
                .iter()
                .find(|metric| metric.metric == MetricKind::TrustSignal)
                .unwrap()
                .score
        );
    }

    #[test]
    fn marketing_simulation_v2_scenario_type_changes_metric_emphasis() {
        let mut request = sample_request_v2();
        request.scenario.scenario_type = ScenarioType::CommunityActivation;

        let result = simulate_marketing_v2(&request).unwrap();
        let first = result.approach_results.first().unwrap();
        let shareability = first
            .primary_scorecard
            .metrics
            .iter()
            .find(|metric| metric.metric == MetricKind::Shareability)
            .unwrap()
            .score;
        let conversion = first
            .primary_scorecard
            .metrics
            .iter()
            .find(|metric| metric.metric == MetricKind::ConversionIntent)
            .unwrap()
            .score;

        assert!(shareability >= conversion);
    }

    #[test]
    fn scenario_family_dynamics_diverge_with_same_seed() {
        let request = sample_request();
        let approach = request.approaches.first().unwrap();
        let seed = 4242;
        let index = 0;
        let time_steps = 6;

        let positioning = simulate_approach(
            &request.seed_data,
            approach,
            &request.platforms,
            request.simulation_size,
            seed,
            index,
            time_steps,
            &ScenarioType::Positioning,
        )
        .unwrap();
        let community = simulate_approach(
            &request.seed_data,
            approach,
            &request.platforms,
            request.simulation_size,
            seed,
            index,
            time_steps,
            &ScenarioType::CommunityActivation,
        )
        .unwrap();
        let retention = simulate_approach(
            &request.seed_data,
            approach,
            &request.platforms,
            request.simulation_size,
            seed,
            index,
            time_steps,
            &ScenarioType::Retention,
        )
        .unwrap();
        let discovery = simulate_approach(
            &request.seed_data,
            approach,
            &request.platforms,
            request.simulation_size,
            seed,
            index,
            time_steps,
            &ScenarioType::AudienceDiscovery,
        )
        .unwrap();

        assert!(positioning.final_means[1] > community.final_means[1]);
        assert!(community.final_means[2] > positioning.final_means[2]);
        assert!(retention.final_means[1] > discovery.final_means[1]);
        assert_ne!(positioning.mean_trajectory, community.mean_trajectory);
    }

    #[test]
    fn marketing_simulation_v2_scenario_type_changes_raw_run_behavior() {
        let mut positioning_request = sample_request_v2();
        positioning_request.approaches = vec![ApproachDefinition {
            id: "community-proof".into(),
            angle: "Show the proof and weekly wins from joining the operator community".into(),
            format: "event recap".into(),
            channels: vec!["twitter".into(), "community".into()],
            tone: "credible and energizing".into(),
            target: "operators who want proof and peers".into(),
            objectives: vec![],
        }];
        positioning_request.scenario.scenario_type = ScenarioType::Positioning;

        let mut community_request = positioning_request.clone();
        community_request.scenario.scenario_type = ScenarioType::CommunityActivation;

        let mut retention_request = positioning_request.clone();
        retention_request.scenario.scenario_type = ScenarioType::Retention;

        let positioning = simulate_marketing_v2(&positioning_request).unwrap();
        let community = simulate_marketing_v2(&community_request).unwrap();
        let retention = simulate_marketing_v2(&retention_request).unwrap();

        let positioning_result = positioning.approach_results.first().unwrap();
        let community_result = community.approach_results.first().unwrap();
        let retention_result = retention.approach_results.first().unwrap();

        assert!(community_result.viral_potential > positioning_result.viral_potential);
        assert!(retention_result.engagement_score != community_result.engagement_score);
        assert_ne!(positioning_result.mean_trajectory, community_result.mean_trajectory);
        assert_ne!(community_result.mean_trajectory, retention_result.mean_trajectory);
    }
}
