use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IdeaPortfolioDraftRequest {
    pub prompt: String,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub population: Option<IdeaPopulationConfig>,
    #[serde(default)]
    pub idea_limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdeaPortfolioRequest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_seed_base")]
    pub seed_base: u64,
    #[serde(default)]
    pub population: IdeaPopulationConfig,
    #[serde(default)]
    pub segments: Vec<IdeaAudienceSegment>,
    #[serde(default)]
    pub ideas: Vec<BusinessIdea>,
    #[serde(default)]
    pub scenarios: Vec<IdeaScenario>,
    #[serde(default)]
    pub evidence: Vec<IdeaEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdeaPopulationConfig {
    #[serde(default = "default_population_size")]
    pub target_count: usize,
    #[serde(default = "default_sample_size")]
    pub sample_size: usize,
}

impl Default for IdeaPopulationConfig {
    fn default() -> Self {
        Self {
            target_count: default_population_size(),
            sample_size: default_sample_size(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdeaAudienceSegment {
    pub id: String,
    pub name: String,
    #[serde(default = "default_share_weight")]
    pub share_weight: f64,
    #[serde(default)]
    pub traits: BTreeMap<String, f64>,
    #[serde(default)]
    pub channels: Vec<String>,
    #[serde(default)]
    pub pains: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessIdea {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub target_segments: Vec<String>,
    #[serde(default)]
    pub channels: Vec<String>,
    #[serde(default)]
    pub trait_weights: BTreeMap<String, f64>,
    #[serde(default)]
    pub strengths: Vec<String>,
    #[serde(default)]
    pub risks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdeaScenario {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub focus: IdeaScenarioFocus,
    #[serde(default)]
    pub channels: Vec<String>,
    #[serde(default = "default_scenario_weight")]
    pub weight: f64,
    #[serde(default = "default_scenario_intensity")]
    pub intensity: f64,
    #[serde(default)]
    pub trait_weights: BTreeMap<String, f64>,
    #[serde(default)]
    pub metrics: Vec<IdeaMetricKind>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Ord, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub enum IdeaScenarioFocus {
    ViralLaunch,
    SocialCommerce,
    Retention,
    PaidConversion,
    BuildSpeed,
    AuthenticityBacklash,
    CommunityReferral,
    LocalIrl,
}

impl Default for IdeaScenarioFocus {
    fn default() -> Self {
        Self::ViralLaunch
    }
}

impl IdeaScenarioFocus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ViralLaunch => "viral_launch",
            Self::SocialCommerce => "social_commerce",
            Self::Retention => "retention",
            Self::PaidConversion => "paid_conversion",
            Self::BuildSpeed => "build_speed",
            Self::AuthenticityBacklash => "authenticity_backlash",
            Self::CommunityReferral => "community_referral",
            Self::LocalIrl => "local_irl",
        }
    }
}

impl std::fmt::Display for IdeaScenarioFocus {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Ord, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub enum IdeaMetricKind {
    MarketPull,
    ViralLoop,
    BuildSpeed,
    AiUnlock,
    ConsumerClarity,
    WillingnessToPay,
    RetentionFit,
    DistributionFit,
    FounderFit,
    RiskResilience,
}

impl IdeaMetricKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::MarketPull => "Market Pull",
            Self::ViralLoop => "Viral Loop",
            Self::BuildSpeed => "Build Speed",
            Self::AiUnlock => "AI Unlock",
            Self::ConsumerClarity => "Consumer Clarity",
            Self::WillingnessToPay => "Willingness To Pay",
            Self::RetentionFit => "Retention Fit",
            Self::DistributionFit => "Distribution Fit",
            Self::FounderFit => "Founder Fit",
            Self::RiskResilience => "Risk Resilience",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdeaEvidence {
    #[serde(default)]
    pub idea_id: Option<String>,
    pub metric: IdeaMetricKind,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    pub value: f64,
    #[serde(default)]
    pub sample_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdeaPortfolioResult {
    pub portfolio_id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub total_population: usize,
    pub seed_base: u64,
    #[serde(default)]
    pub recommended_idea_id: Option<String>,
    #[serde(default)]
    pub top_ranked_idea_id: Option<String>,
    #[serde(default)]
    pub recommendation_status: IdeaRecommendationStatus,
    pub ranked_ideas: Vec<IdeaResult>,
    pub scenario_summaries: Vec<IdeaScenarioSummary>,
    pub sampled_people: Vec<IdeaIndividualSample>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdeaRecommendationStatus {
    /// No usable evidence was attached. The leaderboard is directional only.
    #[default]
    Uncalibrated,
    /// Top-2 score gap is below the kernel's resolution threshold. Treat as tied.
    TiedWithinNoise,
    /// Evidence is attached and the top idea has a clear score lead over its runner-up.
    EvidenceBackedRecommended,
}

/// Score gap (in percent points) below which the top-2 ideas are considered indistinguishable.
/// The portfolio kernel is deterministic, so this is a resolution threshold rather than a
/// statistical confidence interval. Calibrated against typical kernel output spread.
const IDEA_PORTFOLIO_NOISE_THRESHOLD: u32 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdeaResult {
    pub idea_id: String,
    pub idea_name: String,
    pub overall_score: u32,
    pub metrics: Vec<IdeaMetricScore>,
    pub scenario_results: Vec<IdeaScenarioResult>,
    pub strongest_segments: Vec<String>,
    pub weakest_segments: Vec<String>,
    pub evidence_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdeaScenarioResult {
    pub scenario_id: String,
    pub scenario_name: String,
    pub focus: IdeaScenarioFocus,
    pub score: u32,
    pub metrics: Vec<IdeaMetricScore>,
    pub segment_results: Vec<IdeaSegmentResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdeaScenarioSummary {
    pub scenario_id: String,
    pub scenario_name: String,
    pub focus: IdeaScenarioFocus,
    pub top_idea_id: String,
    pub top_score: u32,
    #[serde(default)]
    pub runner_up_idea_id: Option<String>,
    #[serde(default)]
    pub score_gap_vs_runner_up: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdeaSegmentResult {
    pub segment_id: String,
    pub segment_name: String,
    pub buyers: usize,
    pub score: u32,
    pub adoption_probability: u32,
    pub share_probability: u32,
    pub pay_probability: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdeaMetricScore {
    pub metric: IdeaMetricKind,
    pub label: String,
    pub score: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdeaIndividualSample {
    pub individual_id: String,
    pub segment_id: String,
    pub top_idea_id: String,
    pub top_idea_score: u32,
    #[serde(default)]
    pub runner_up_idea_id: Option<String>,
    #[serde(default)]
    pub runner_up_score: Option<u32>,
    pub adoption_probability: u32,
    pub share_probability: u32,
    pub pay_probability: u32,
}

#[derive(Debug, Clone)]
struct ScenarioScore {
    score: f64,
    metrics: BTreeMap<IdeaMetricKind, f64>,
    segment_results: Vec<IdeaSegmentResult>,
}

#[derive(Debug, Clone)]
struct IndividualIdeaScore {
    idea_id: String,
    score: f64,
    adoption_probability: f64,
    share_probability: f64,
    pay_probability: f64,
}

pub fn draft_idea_portfolio(
    request: &IdeaPortfolioDraftRequest,
) -> Result<IdeaPortfolioRequest, IdeaPortfolioDraftError> {
    let prompt = request.prompt.trim();
    if prompt.is_empty() {
        return Err(IdeaPortfolioDraftError::EmptyPrompt);
    }

    let ideas = extract_ideas(prompt, request.idea_limit.unwrap_or(50));
    if ideas.is_empty() {
        return Err(IdeaPortfolioDraftError::NoIdeasFound);
    }

    let name = request
        .name
        .clone()
        .unwrap_or_else(|| "AI Consumer Idea Portfolio".into());
    let id = request
        .id
        .clone()
        .unwrap_or_else(|| slugify_non_empty(&name, "idea-portfolio"));
    let seed_base = stable_text_seed("idea_portfolio_draft_v1", prompt, &id);
    let portfolio = IdeaPortfolioRequest {
        id,
        name,
        description: request
            .description
            .clone()
            .or_else(|| Some(prompt_excerpt(prompt, 260))),
        seed_base,
        population: request.population.clone().unwrap_or_default(),
        segments: default_consumer_ai_segments(),
        ideas,
        scenarios: default_consumer_ai_scenarios(),
        evidence: default_market_evidence(),
    };

    validate_portfolio_request(&portfolio).map_err(IdeaPortfolioDraftError::Portfolio)?;
    Ok(portfolio)
}

pub fn run_idea_portfolio(
    request: &IdeaPortfolioRequest,
) -> Result<IdeaPortfolioResult, IdeaPortfolioError> {
    validate_portfolio_request(request)?;

    let segment_counts = segment_counts(&request.segments, request.population.target_count);
    let mut ranked_ideas = request
        .ideas
        .iter()
        .map(|idea| {
            let scenario_results = request
                .scenarios
                .iter()
                .map(|scenario| {
                    score_idea_in_scenario(
                        idea,
                        scenario,
                        &request.segments,
                        &segment_counts,
                        &request.evidence,
                    )
                })
                .collect::<Vec<_>>();

            let total_weight = request
                .scenarios
                .iter()
                .map(|scenario| scenario.weight.max(0.0))
                .sum::<f64>()
                .max(1.0);
            let overall = scenario_results
                .iter()
                .zip(request.scenarios.iter())
                .map(|(score, scenario)| score.score * scenario.weight.max(0.0))
                .sum::<f64>()
                / total_weight;

            let mut aggregate_metrics = BTreeMap::<IdeaMetricKind, Vec<f64>>::new();
            let mut aggregate_segments = BTreeMap::<String, Vec<u32>>::new();
            let public_scenario_results = scenario_results
                .iter()
                .zip(request.scenarios.iter())
                .map(|(score, scenario)| {
                    for (metric, value) in &score.metrics {
                        aggregate_metrics.entry(*metric).or_default().push(*value);
                    }
                    for segment in &score.segment_results {
                        aggregate_segments
                            .entry(segment.segment_id.clone())
                            .or_default()
                            .push(segment.score);
                    }
                    IdeaScenarioResult {
                        scenario_id: scenario.id.clone(),
                        scenario_name: scenario.name.clone(),
                        focus: scenario.focus,
                        score: as_percent(score.score),
                        metrics: metric_scores(&score.metrics),
                        segment_results: score.segment_results.clone(),
                    }
                })
                .collect::<Vec<_>>();

            let metrics = aggregate_metrics
                .into_iter()
                .map(|(metric, values)| IdeaMetricScore {
                    metric,
                    label: metric.label().into(),
                    score: as_percent(values.iter().sum::<f64>() / values.len() as f64),
                })
                .collect::<Vec<_>>();

            let mut segment_rankings = aggregate_segments
                .into_iter()
                .map(|(segment_id, scores)| {
                    let average =
                        scores.iter().map(|value| *value as f64).sum::<f64>() / scores.len() as f64;
                    (segment_id, average.round() as u32)
                })
                .collect::<Vec<_>>();
            segment_rankings
                .sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
            let strongest_segments = segment_rankings
                .iter()
                .take(2)
                .map(|(segment_id, _)| segment_id.clone())
                .collect();
            let weakest_segments = segment_rankings
                .iter()
                .rev()
                .take(2)
                .map(|(segment_id, _)| segment_id.clone())
                .collect();

            IdeaResult {
                idea_id: idea.id.clone(),
                idea_name: idea.name.clone(),
                overall_score: as_percent(overall),
                metrics,
                scenario_results: public_scenario_results,
                strongest_segments,
                weakest_segments,
                evidence_notes: evidence_notes(idea, &request.evidence),
            }
        })
        .collect::<Vec<_>>();

    ranked_ideas.sort_by(|left, right| {
        right
            .overall_score
            .cmp(&left.overall_score)
            .then_with(|| left.idea_id.cmp(&right.idea_id))
    });

    let scenario_summaries = build_scenario_summaries(&request.scenarios, &ranked_ideas);
    let sampled_people = sample_people(request, &segment_counts);
    let top_ranked_idea_id = ranked_ideas.first().map(|idea| idea.idea_id.clone());
    let recommendation_status =
        compute_idea_recommendation_status(&ranked_ideas, &request.evidence);
    let recommended_idea_id = if matches!(
        recommendation_status,
        IdeaRecommendationStatus::EvidenceBackedRecommended
    ) {
        top_ranked_idea_id.clone()
    } else {
        None
    };
    let notes = build_notes(
        request,
        &ranked_ideas,
        &scenario_summaries,
        &recommendation_status,
    );

    Ok(IdeaPortfolioResult {
        portfolio_id: request.id.clone(),
        name: request.name.clone(),
        description: request.description.clone(),
        total_population: request.population.target_count,
        seed_base: request.seed_base,
        recommended_idea_id,
        top_ranked_idea_id,
        recommendation_status,
        ranked_ideas,
        scenario_summaries,
        sampled_people,
        notes,
    })
}

fn compute_idea_recommendation_status(
    ranked_ideas: &[IdeaResult],
    evidence: &[IdeaEvidence],
) -> IdeaRecommendationStatus {
    let has_usable_evidence = evidence
        .iter()
        .any(|item| item.value.is_finite() && (0.0..=1.0).contains(&item.value));
    if !has_usable_evidence {
        return IdeaRecommendationStatus::Uncalibrated;
    }
    match (ranked_ideas.first(), ranked_ideas.get(1)) {
        (Some(top), Some(runner_up))
            if top.overall_score.saturating_sub(runner_up.overall_score)
                < IDEA_PORTFOLIO_NOISE_THRESHOLD =>
        {
            IdeaRecommendationStatus::TiedWithinNoise
        }
        _ => IdeaRecommendationStatus::EvidenceBackedRecommended,
    }
}

fn score_idea_in_scenario(
    idea: &BusinessIdea,
    scenario: &IdeaScenario,
    segments: &[IdeaAudienceSegment],
    segment_counts: &BTreeMap<String, usize>,
    evidence: &[IdeaEvidence],
) -> ScenarioScore {
    let mut metric_sums = BTreeMap::<IdeaMetricKind, f64>::new();
    let mut score_sum = 0.0;
    let mut buyers_sum = 0usize;
    let mut segment_results = Vec::new();

    for segment in segments {
        let buyers = segment_counts.get(&segment.id).copied().unwrap_or_default();
        if buyers == 0 {
            continue;
        }
        let segment_metrics = score_metrics(idea, scenario, segment, evidence);
        let score = focus_weighted_score(scenario.focus, &segment_metrics);
        let share_probability = segment_metrics
            .get(&IdeaMetricKind::ViralLoop)
            .copied()
            .unwrap_or(0.5)
            * trait_value(segment, "social_sharing");
        let pay_probability = segment_metrics
            .get(&IdeaMetricKind::WillingnessToPay)
            .copied()
            .unwrap_or(0.5)
            * 0.72;
        score_sum += score * buyers as f64;
        buyers_sum += buyers;
        for (metric, value) in &segment_metrics {
            *metric_sums.entry(*metric).or_default() += *value * buyers as f64;
        }
        segment_results.push(IdeaSegmentResult {
            segment_id: segment.id.clone(),
            segment_name: segment.name.clone(),
            buyers,
            score: as_percent(score),
            adoption_probability: as_percent(adoption_probability(score, idea, scenario, segment)),
            share_probability: as_percent(share_probability),
            pay_probability: as_percent(pay_probability),
        });
    }

    let buyers = buyers_sum.max(1) as f64;
    let metrics = metric_sums
        .into_iter()
        .map(|(metric, value)| (metric, clamp01(value / buyers)))
        .collect::<BTreeMap<_, _>>();
    ScenarioScore {
        score: clamp01(score_sum / buyers),
        metrics,
        segment_results,
    }
}

fn score_metrics(
    idea: &BusinessIdea,
    scenario: &IdeaScenario,
    segment: &IdeaAudienceSegment,
    evidence: &[IdeaEvidence],
) -> BTreeMap<IdeaMetricKind, f64> {
    let text = idea_text(idea);
    let segment_fit = segment_fit(idea, segment);
    let trait_fit = weighted_trait_fit(&idea.trait_weights, &segment.traits).unwrap_or(0.55);
    let scenario_fit = weighted_trait_fit(&scenario.trait_weights, &idea.trait_weights)
        .or_else(|| weighted_trait_fit(&scenario.trait_weights, &segment.traits))
        .unwrap_or(0.55);
    let channel_fit = channel_fit(&idea.channels, &segment.channels, &scenario.channels);
    let pain_match = overlap_text(&text, &segment.pains);
    let viral_keyword = keyword_score(
        &text,
        &[
            "viral",
            "share",
            "creator",
            "friend",
            "meme",
            "ugc",
            "tiktok",
            "instagram",
            "community",
        ],
    );
    let ai_keyword = keyword_score(
        &text,
        &[
            "ai",
            "agent",
            "codex",
            "gpt",
            "image",
            "video",
            "voice",
            "seedance",
            "suno",
            "browser",
            "automation",
        ],
    );
    let retention_keyword = keyword_score(
        &text,
        &[
            "daily",
            "weekly",
            "streak",
            "cohort",
            "season",
            "habit",
            "recurring",
            "league",
            "loop",
        ],
    );
    let founder_keyword = keyword_score(
        &text,
        &[
            "forge",
            "metal",
            "ios",
            "airspace",
            "sentinel",
            "composure",
            "game",
            "voice",
            "map",
            "agent",
        ],
    );
    let risk_penalty = risk_penalty(idea);

    let mut metrics = BTreeMap::new();
    for metric in all_metrics() {
        let evidence_score = evidence_score(idea, metric, evidence).unwrap_or(0.50);
        let value = match metric {
            IdeaMetricKind::MarketPull => {
                0.22 + (segment_fit * 0.20)
                    + (trait_fit * 0.18)
                    + (pain_match * 0.14)
                    + (evidence_score * 0.18)
                    + (scenario_fit * 0.08)
            }
            IdeaMetricKind::ViralLoop => {
                0.14 + (trait_value(segment, "social_sharing") * 0.20)
                    + (channel_fit * 0.20)
                    + (viral_keyword * 0.24)
                    + (evidence_score * 0.14)
                    + (scenario_fit * 0.08)
            }
            IdeaMetricKind::BuildSpeed => {
                0.62 + (keyword_score(&text, &["simple", "prompt", "pipeline", "agent"]) * 0.12)
                    + (founder_keyword * 0.16)
                    - (risk_penalty * 0.24)
            }
            IdeaMetricKind::AiUnlock => {
                0.18 + (ai_keyword * 0.46)
                    + (trait_fit * 0.12)
                    + (evidence_score * 0.16)
                    + (founder_keyword * 0.08)
            }
            IdeaMetricKind::ConsumerClarity => {
                0.24 + (segment_fit * 0.18)
                    + (keyword_score(&text, &["one tap", "upload", "text", "photo"]) * 0.18)
                    + (summary_clarity(idea) * 0.24)
                    + (evidence_score * 0.16)
            }
            IdeaMetricKind::WillingnessToPay => {
                0.18 + (trait_value(segment, "spending_power") * 0.22)
                    + (pain_match * 0.16)
                    + (keyword_score(&text, &["saves", "books", "refund", "business"]) * 0.16)
                    + (evidence_score * 0.20)
                    + (trait_fit * 0.08)
            }
            IdeaMetricKind::RetentionFit => {
                0.18 + (retention_keyword * 0.32)
                    + (trait_value(segment, "habit_openness") * 0.16)
                    + (scenario_fit * 0.12)
                    + (evidence_score * 0.14)
                    - (risk_penalty * 0.12)
            }
            IdeaMetricKind::DistributionFit => {
                0.18 + (channel_fit * 0.38)
                    + (viral_keyword * 0.18)
                    + (trait_value(segment, "social_sharing") * 0.12)
                    + (evidence_score * 0.14)
            }
            IdeaMetricKind::FounderFit => {
                0.20 + (founder_keyword * 0.44)
                    + (ai_keyword * 0.12)
                    + (keyword_score(&text, &["game", "agent", "ios", "voice"]) * 0.16)
                    + (evidence_score * 0.08)
            }
            IdeaMetricKind::RiskResilience => {
                0.72 - (risk_penalty * 0.42)
                    + (keyword_score(&text, &["proof", "human", "authentic", "trust"]) * 0.12)
                    + (evidence_score * 0.16)
            }
        };
        metrics.insert(metric, clamp01(value * scenario.intensity.clamp(0.4, 1.4)));
    }
    metrics
}

fn focus_weighted_score(focus: IdeaScenarioFocus, metrics: &BTreeMap<IdeaMetricKind, f64>) -> f64 {
    let weights: &[(IdeaMetricKind, f64)] = match focus {
        IdeaScenarioFocus::ViralLaunch => &[
            (IdeaMetricKind::MarketPull, 0.18),
            (IdeaMetricKind::ViralLoop, 0.26),
            (IdeaMetricKind::ConsumerClarity, 0.18),
            (IdeaMetricKind::DistributionFit, 0.20),
            (IdeaMetricKind::AiUnlock, 0.10),
            (IdeaMetricKind::RiskResilience, 0.08),
        ],
        IdeaScenarioFocus::SocialCommerce => &[
            (IdeaMetricKind::MarketPull, 0.18),
            (IdeaMetricKind::ViralLoop, 0.18),
            (IdeaMetricKind::WillingnessToPay, 0.24),
            (IdeaMetricKind::DistributionFit, 0.20),
            (IdeaMetricKind::ConsumerClarity, 0.12),
            (IdeaMetricKind::RiskResilience, 0.08),
        ],
        IdeaScenarioFocus::Retention => &[
            (IdeaMetricKind::RetentionFit, 0.34),
            (IdeaMetricKind::MarketPull, 0.16),
            (IdeaMetricKind::ConsumerClarity, 0.14),
            (IdeaMetricKind::WillingnessToPay, 0.14),
            (IdeaMetricKind::RiskResilience, 0.14),
            (IdeaMetricKind::FounderFit, 0.08),
        ],
        IdeaScenarioFocus::PaidConversion => &[
            (IdeaMetricKind::WillingnessToPay, 0.30),
            (IdeaMetricKind::ConsumerClarity, 0.18),
            (IdeaMetricKind::MarketPull, 0.18),
            (IdeaMetricKind::RiskResilience, 0.16),
            (IdeaMetricKind::DistributionFit, 0.10),
            (IdeaMetricKind::BuildSpeed, 0.08),
        ],
        IdeaScenarioFocus::BuildSpeed => &[
            (IdeaMetricKind::BuildSpeed, 0.34),
            (IdeaMetricKind::FounderFit, 0.24),
            (IdeaMetricKind::AiUnlock, 0.18),
            (IdeaMetricKind::ConsumerClarity, 0.10),
            (IdeaMetricKind::MarketPull, 0.08),
            (IdeaMetricKind::RiskResilience, 0.06),
        ],
        IdeaScenarioFocus::AuthenticityBacklash => &[
            (IdeaMetricKind::RiskResilience, 0.30),
            (IdeaMetricKind::ConsumerClarity, 0.18),
            (IdeaMetricKind::MarketPull, 0.16),
            (IdeaMetricKind::RetentionFit, 0.14),
            (IdeaMetricKind::DistributionFit, 0.12),
            (IdeaMetricKind::AiUnlock, 0.10),
        ],
        IdeaScenarioFocus::CommunityReferral => &[
            (IdeaMetricKind::ViralLoop, 0.28),
            (IdeaMetricKind::RetentionFit, 0.20),
            (IdeaMetricKind::DistributionFit, 0.18),
            (IdeaMetricKind::MarketPull, 0.14),
            (IdeaMetricKind::ConsumerClarity, 0.12),
            (IdeaMetricKind::RiskResilience, 0.08),
        ],
        IdeaScenarioFocus::LocalIrl => &[
            (IdeaMetricKind::MarketPull, 0.20),
            (IdeaMetricKind::ConsumerClarity, 0.18),
            (IdeaMetricKind::WillingnessToPay, 0.18),
            (IdeaMetricKind::DistributionFit, 0.16),
            (IdeaMetricKind::RetentionFit, 0.16),
            (IdeaMetricKind::RiskResilience, 0.12),
        ],
    };

    let total_weight = weights
        .iter()
        .map(|(_, weight)| weight)
        .sum::<f64>()
        .max(1.0);
    let weighted = weights
        .iter()
        .map(|(metric, weight)| metrics.get(metric).copied().unwrap_or(0.5) * weight)
        .sum::<f64>()
        / total_weight;
    let min_metric = weights
        .iter()
        .map(|(metric, _)| metrics.get(metric).copied().unwrap_or(0.5))
        .fold(1.0_f64, f64::min)
        .clamp(0.0, 1.0);
    let floor = 0.65 + 0.35 * min_metric;
    clamp01(weighted * floor)
}

fn build_scenario_summaries(
    scenarios: &[IdeaScenario],
    ranked_ideas: &[IdeaResult],
) -> Vec<IdeaScenarioSummary> {
    scenarios
        .iter()
        .filter_map(|scenario| {
            let mut ranked = ranked_ideas
                .iter()
                .filter_map(|idea| {
                    idea.scenario_results
                        .iter()
                        .find(|result| result.scenario_id == scenario.id)
                        .map(|result| (idea.idea_id.clone(), result.score))
                })
                .collect::<Vec<_>>();
            ranked.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
            let top = ranked.first()?;
            let runner_up = ranked.get(1);
            Some(IdeaScenarioSummary {
                scenario_id: scenario.id.clone(),
                scenario_name: scenario.name.clone(),
                focus: scenario.focus,
                top_idea_id: top.0.clone(),
                top_score: top.1,
                runner_up_idea_id: runner_up.map(|value| value.0.clone()),
                score_gap_vs_runner_up: runner_up.map(|value| top.1 as i32 - value.1 as i32),
            })
        })
        .collect()
}

fn sample_people(
    request: &IdeaPortfolioRequest,
    segment_counts: &BTreeMap<String, usize>,
) -> Vec<IdeaIndividualSample> {
    if request.population.sample_size == 0 || request.ideas.is_empty() {
        return Vec::new();
    }

    let mut samples = Vec::new();
    let mut segment_cycle = request
        .segments
        .iter()
        .filter(|segment| segment_counts.get(&segment.id).copied().unwrap_or_default() > 0)
        .collect::<Vec<_>>();
    if segment_cycle.is_empty() {
        return Vec::new();
    }
    segment_cycle.sort_by(|left, right| left.id.cmp(&right.id));
    let default_scenario = request.scenarios.first();

    for index in 0..request.population.sample_size {
        let segment = segment_cycle[index % segment_cycle.len()];
        let mut scored = request
            .ideas
            .iter()
            .map(|idea| {
                score_individual(
                    idea,
                    segment,
                    default_scenario,
                    &request.evidence,
                    request.seed_base,
                    index,
                )
            })
            .collect::<Vec<_>>();
        scored.sort_by(|left, right| {
            right
                .score
                .total_cmp(&left.score)
                .then_with(|| left.idea_id.cmp(&right.idea_id))
        });
        let Some(top) = scored.first() else {
            continue;
        };
        let runner_up = scored.get(1);
        samples.push(IdeaIndividualSample {
            individual_id: format!("idea-person-{:04}", index + 1),
            segment_id: segment.id.clone(),
            top_idea_id: top.idea_id.clone(),
            top_idea_score: as_percent(top.score),
            runner_up_idea_id: runner_up.map(|value| value.idea_id.clone()),
            runner_up_score: runner_up.map(|value| as_percent(value.score)),
            adoption_probability: as_percent(top.adoption_probability),
            share_probability: as_percent(top.share_probability),
            pay_probability: as_percent(top.pay_probability),
        });
    }
    samples
}

fn score_individual(
    idea: &BusinessIdea,
    segment: &IdeaAudienceSegment,
    scenario: Option<&IdeaScenario>,
    evidence: &[IdeaEvidence],
    seed_base: u64,
    index: usize,
) -> IndividualIdeaScore {
    let fallback_scenario = IdeaScenario {
        id: "default".into(),
        name: "Default".into(),
        description: None,
        focus: IdeaScenarioFocus::ViralLaunch,
        channels: Vec::new(),
        weight: 1.0,
        intensity: 1.0,
        trait_weights: BTreeMap::new(),
        metrics: all_metrics(),
    };
    let scenario = scenario.unwrap_or(&fallback_scenario);
    let metrics = score_metrics(idea, scenario, segment, evidence);
    let noise = stable_noise(seed_base, &segment.id, &idea.id, index);
    let score = clamp01((focus_weighted_score(scenario.focus, &metrics) * 0.88) + (noise * 0.12));
    IndividualIdeaScore {
        idea_id: idea.id.clone(),
        score,
        adoption_probability: adoption_probability(score, idea, scenario, segment),
        share_probability: clamp01(
            metrics
                .get(&IdeaMetricKind::ViralLoop)
                .copied()
                .unwrap_or(0.5)
                * trait_value(segment, "social_sharing"),
        ),
        pay_probability: clamp01(
            metrics
                .get(&IdeaMetricKind::WillingnessToPay)
                .copied()
                .unwrap_or(0.5)
                * 0.72,
        ),
    }
}

fn build_notes(
    request: &IdeaPortfolioRequest,
    ranked_ideas: &[IdeaResult],
    scenario_summaries: &[IdeaScenarioSummary],
    recommendation_status: &IdeaRecommendationStatus,
) -> Vec<String> {
    let mut notes = Vec::new();
    notes.push(format!(
        "Portfolio tested `{}` ideas across `{}` scenarios and `{}` audience segments.",
        request.ideas.len(),
        request.scenarios.len(),
        request.segments.len()
    ));
    match recommendation_status {
        IdeaRecommendationStatus::Uncalibrated => {
            notes.push(
                "No usable evidence was attached, so this is a directional prioritization pass — use it to choose what to test next, not as a forecast."
                    .into(),
            );
        }
        IdeaRecommendationStatus::TiedWithinNoise => {
            notes.push(format!(
                "Top-2 ideas score within `{}` points of each other, which is below the kernel's resolution threshold. Treat as tied — pick the next real-world signal to gather instead of declaring a winner.",
                IDEA_PORTFOLIO_NOISE_THRESHOLD
            ));
        }
        IdeaRecommendationStatus::EvidenceBackedRecommended => {}
    }
    if let Some(winner) = ranked_ideas.first() {
        let label = match recommendation_status {
            IdeaRecommendationStatus::EvidenceBackedRecommended => "is the recommended idea",
            _ => "is the current top-ranked idea",
        };
        notes.push(format!(
            "`{}` {label} with score `{}`.",
            winner.idea_id, winner.overall_score
        ));
    }
    if let Some(tightest) = scenario_summaries
        .iter()
        .filter(|summary| summary.score_gap_vs_runner_up.is_some())
        .min_by_key(|summary| summary.score_gap_vs_runner_up.unwrap_or(i32::MAX))
    {
        notes.push(format!(
            "`{}` is the least decisive scenario; gather more evidence there.",
            tightest.scenario_name
        ));
    }
    notes
}

fn metric_scores(metrics: &BTreeMap<IdeaMetricKind, f64>) -> Vec<IdeaMetricScore> {
    all_metrics()
        .into_iter()
        .map(|metric| IdeaMetricScore {
            metric,
            label: metric.label().into(),
            score: as_percent(metrics.get(&metric).copied().unwrap_or(0.5)),
        })
        .collect()
}

fn extract_ideas(prompt: &str, limit: usize) -> Vec<BusinessIdea> {
    let mut ideas = Vec::new();
    for line in prompt.lines() {
        if let Some((name, summary)) = parse_table_idea_line(line) {
            ideas.push(draft_business_idea(&name, summary.as_deref()));
            continue;
        }
        if let Some((name, summary)) = parse_numbered_idea_line(line) {
            ideas.push(draft_business_idea(&name, summary.as_deref()));
        }
    }

    dedupe_ideas(ideas).into_iter().take(limit.max(1)).collect()
}

fn parse_table_idea_line(line: &str) -> Option<(String, Option<String>)> {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') || trimmed.contains("---") {
        return None;
    }
    let columns = trimmed
        .split('|')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if columns.len() < 3 || columns[0].parse::<usize>().is_err() {
        return None;
    }
    Some((columns[1].to_string(), Some(columns[2].to_string())))
}

fn parse_numbered_idea_line(line: &str) -> Option<(String, Option<String>)> {
    let trimmed = line.trim();
    let (number, rest) = trimmed.split_once('.')?;
    if number.trim().parse::<usize>().is_err() {
        return None;
    }
    let rest = rest.trim();
    if rest.is_empty() {
        return None;
    }
    let split = rest
        .split_once(" — ")
        .or_else(|| rest.split_once(" - "))
        .or_else(|| rest.split_once(": "));
    match split {
        Some((name, summary)) => Some((name.trim().to_string(), Some(summary.trim().to_string()))),
        None => Some((rest.to_string(), None)),
    }
}

fn draft_business_idea(name: &str, summary: Option<&str>) -> BusinessIdea {
    let text = [name, summary.unwrap_or("")]
        .into_iter()
        .collect::<Vec<_>>()
        .join(" ");
    BusinessIdea {
        id: slugify_non_empty(name, "idea"),
        name: name.trim().to_string(),
        summary: summary.map(prompt_excerpt_owned),
        category: inferred_category(&text),
        target_segments: inferred_target_segments(&text),
        channels: inferred_channels(&text),
        trait_weights: inferred_trait_weights(&text),
        strengths: inferred_strengths(&text),
        risks: inferred_risks(&text),
    }
}

fn dedupe_ideas(ideas: Vec<BusinessIdea>) -> Vec<BusinessIdea> {
    let mut seen = BTreeMap::<String, usize>::new();
    ideas
        .into_iter()
        .map(|mut idea| {
            let count = seen.entry(idea.id.clone()).or_default();
            if *count > 0 {
                idea.id = format!("{}-{}", idea.id, *count + 1);
            }
            *count += 1;
            idea
        })
        .collect()
}

fn default_consumer_ai_segments() -> Vec<IdeaAudienceSegment> {
    vec![
        segment(
            "gen_z_social_creators",
            "Gen Z Social Creators",
            0.24,
            &[
                ("social_sharing", 0.92),
                ("trend_adoption", 0.88),
                ("spending_power", 0.46),
                ("habit_openness", 0.72),
                ("ai_openness", 0.86),
            ],
            &["tiktok", "instagram", "x", "discord"],
            &["needs fresh identity loops", "hates obvious ai slop"],
        ),
        segment(
            "creator_operators",
            "Creator Operators",
            0.20,
            &[
                ("social_sharing", 0.82),
                ("trend_adoption", 0.78),
                ("spending_power", 0.74),
                ("habit_openness", 0.66),
                ("ai_openness", 0.92),
            ],
            &["tiktok", "youtube", "instagram", "newsletter"],
            &["needs monetizable output", "needs faster production"],
        ),
        segment(
            "local_business_owners",
            "Local Business Owners",
            0.16,
            &[
                ("social_sharing", 0.56),
                ("trend_adoption", 0.58),
                ("spending_power", 0.82),
                ("habit_openness", 0.54),
                ("ai_openness", 0.66),
            ],
            &["instagram", "google", "facebook", "tiktok"],
            &["needs customers now", "has no time for content"],
        ),
        segment(
            "parents_and_families",
            "Parents And Families",
            0.14,
            &[
                ("social_sharing", 0.70),
                ("trend_adoption", 0.52),
                ("spending_power", 0.76),
                ("habit_openness", 0.80),
                ("ai_openness", 0.58),
            ],
            &["instagram", "facebook", "imessage", "tiktok"],
            &[
                "needs emotional keepsakes",
                "needs trusted child-safe tools",
            ],
        ),
        segment(
            "gamers_and_builders",
            "Gamers And Builders",
            0.16,
            &[
                ("social_sharing", 0.78),
                ("trend_adoption", 0.74),
                ("spending_power", 0.62),
                ("habit_openness", 0.84),
                ("ai_openness", 0.90),
            ],
            &["discord", "tiktok", "youtube", "app_store"],
            &["wants playable creation", "rejects shallow novelty"],
        ),
        segment(
            "dating_app_fatigued",
            "Dating App Fatigued Adults",
            0.10,
            &[
                ("social_sharing", 0.62),
                ("trend_adoption", 0.62),
                ("spending_power", 0.70),
                ("habit_openness", 0.68),
                ("ai_openness", 0.54),
            ],
            &["instagram", "sms", "events", "tiktok"],
            &["wants irl connection", "does not trust fake profiles"],
        ),
    ]
}

fn default_consumer_ai_scenarios() -> Vec<IdeaScenario> {
    vec![
        scenario(
            "tiktok_ig_viral_launch",
            "TikTok/IG Viral Launch",
            IdeaScenarioFocus::ViralLaunch,
            &["tiktok", "instagram", "x"],
            &[("social_sharing", 1.0), ("trend_adoption", 0.8)],
        ),
        scenario(
            "social_commerce_conversion",
            "Social Commerce Conversion",
            IdeaScenarioFocus::SocialCommerce,
            &["tiktok", "instagram", "shopify"],
            &[("spending_power", 0.9), ("ai_openness", 0.4)],
        ),
        scenario(
            "novelty_decay_retention",
            "Novelty Decay Retention",
            IdeaScenarioFocus::Retention,
            &["app_store", "imessage", "discord"],
            &[("habit_openness", 1.0), ("ai_openness", 0.5)],
        ),
        scenario(
            "paid_offer_pressure",
            "Paid Offer Pressure",
            IdeaScenarioFocus::PaidConversion,
            &["landing_page", "app_store", "email"],
            &[("spending_power", 1.0)],
        ),
        scenario(
            "tiny_team_build_speed",
            "Tiny Team Build Speed",
            IdeaScenarioFocus::BuildSpeed,
            &["app_store", "web", "api"],
            &[("ai_openness", 1.0)],
        ),
        scenario(
            "ai_slop_backlash",
            "AI Slop Backlash",
            IdeaScenarioFocus::AuthenticityBacklash,
            &["tiktok", "instagram", "x"],
            &[("trend_adoption", 0.5), ("social_sharing", 0.5)],
        ),
        scenario(
            "community_referral_loop",
            "Community Referral Loop",
            IdeaScenarioFocus::CommunityReferral,
            &["discord", "imessage", "instagram", "events"],
            &[("social_sharing", 1.0), ("habit_openness", 0.7)],
        ),
    ]
}

fn default_market_evidence() -> Vec<IdeaEvidence> {
    vec![
        evidence(
            None,
            IdeaMetricKind::ViralLoop,
            0.82,
            "Public trend scan",
            "Short-form social remains the distribution layer for consumer launches.",
        ),
        evidence(
            None,
            IdeaMetricKind::AiUnlock,
            0.86,
            "AI tooling scan",
            "Coding agents, image/video generation, voice, and browser agents lower launch cost.",
        ),
        evidence(
            None,
            IdeaMetricKind::RiskResilience,
            0.48,
            "AI slop backlash",
            "Generated novelty alone decays quickly unless tied to human proof, identity, or utility.",
        ),
        evidence(
            None,
            IdeaMetricKind::WillingnessToPay,
            0.68,
            "Creator/social commerce benchmarks",
            "Creator, commerce, and utility workflows show stronger monetization than generic toys.",
        ),
    ]
}

fn segment(
    id: &str,
    name: &str,
    share_weight: f64,
    traits: &[(&str, f64)],
    channels: &[&str],
    pains: &[&str],
) -> IdeaAudienceSegment {
    IdeaAudienceSegment {
        id: id.into(),
        name: name.into(),
        share_weight,
        traits: traits
            .iter()
            .map(|(name, value)| ((*name).into(), *value))
            .collect(),
        channels: channels.iter().map(|value| (*value).into()).collect(),
        pains: pains.iter().map(|value| (*value).into()).collect(),
    }
}

fn scenario(
    id: &str,
    name: &str,
    focus: IdeaScenarioFocus,
    channels: &[&str],
    trait_weights: &[(&str, f64)],
) -> IdeaScenario {
    IdeaScenario {
        id: id.into(),
        name: name.into(),
        description: None,
        focus,
        channels: channels.iter().map(|value| (*value).into()).collect(),
        weight: 1.0,
        intensity: 1.0,
        trait_weights: trait_weights
            .iter()
            .map(|(name, value)| ((*name).into(), *value))
            .collect(),
        metrics: all_metrics(),
    }
}

fn evidence(
    idea_id: Option<&str>,
    metric: IdeaMetricKind,
    value: f64,
    source: &str,
    note: &str,
) -> IdeaEvidence {
    IdeaEvidence {
        idea_id: idea_id.map(str::to_string),
        metric,
        source: Some(source.into()),
        note: Some(note.into()),
        value,
        sample_size: None,
    }
}

fn validate_portfolio_request(request: &IdeaPortfolioRequest) -> Result<(), IdeaPortfolioError> {
    validate_non_empty(&request.id, IdeaPortfolioError::EmptyId)?;
    validate_non_empty(&request.name, IdeaPortfolioError::EmptyName)?;
    if request.population.target_count == 0 {
        return Err(IdeaPortfolioError::ZeroPopulation);
    }
    if request.segments.is_empty() {
        return Err(IdeaPortfolioError::NoSegments);
    }
    if request.ideas.is_empty() {
        return Err(IdeaPortfolioError::NoIdeas);
    }
    if request.scenarios.is_empty() {
        return Err(IdeaPortfolioError::NoScenarios);
    }

    let mut segment_ids = BTreeSet::new();
    for segment in &request.segments {
        validate_non_empty(&segment.id, IdeaPortfolioError::EmptySegmentId)?;
        validate_non_empty(&segment.name, IdeaPortfolioError::EmptySegmentName)?;
        if !segment_ids.insert(segment.id.clone()) {
            return Err(IdeaPortfolioError::DuplicateSegmentId(segment.id.clone()));
        }
        validate_positive_finite(segment.share_weight, || {
            IdeaPortfolioError::InvalidSegmentShare {
                segment_id: segment.id.clone(),
                value: segment.share_weight,
            }
        })?;
        validate_trait_values(&segment.id, &segment.traits)?;
    }

    let mut idea_ids = BTreeSet::new();
    for idea in &request.ideas {
        validate_non_empty(&idea.id, IdeaPortfolioError::EmptyIdeaId)?;
        validate_non_empty(&idea.name, IdeaPortfolioError::EmptyIdeaName)?;
        if !idea_ids.insert(idea.id.clone()) {
            return Err(IdeaPortfolioError::DuplicateIdeaId(idea.id.clone()));
        }
        for segment_id in &idea.target_segments {
            if !segment_ids.contains(segment_id) {
                return Err(IdeaPortfolioError::UnknownIdeaSegment {
                    idea_id: idea.id.clone(),
                    segment_id: segment_id.clone(),
                });
            }
        }
        for (trait_name, value) in &idea.trait_weights {
            validate_non_empty(
                trait_name,
                IdeaPortfolioError::InvalidIdeaTraitName(idea.id.clone()),
            )?;
            if !value.is_finite() {
                return Err(IdeaPortfolioError::InvalidIdeaTraitWeight {
                    idea_id: idea.id.clone(),
                    trait_name: trait_name.clone(),
                    value: *value,
                });
            }
        }
    }

    let mut scenario_ids = BTreeSet::new();
    for scenario in &request.scenarios {
        validate_non_empty(&scenario.id, IdeaPortfolioError::EmptyScenarioId)?;
        validate_non_empty(&scenario.name, IdeaPortfolioError::EmptyScenarioName)?;
        if !scenario_ids.insert(scenario.id.clone()) {
            return Err(IdeaPortfolioError::DuplicateScenarioId(scenario.id.clone()));
        }
        validate_positive_finite(scenario.weight, || {
            IdeaPortfolioError::InvalidScenarioWeight {
                scenario_id: scenario.id.clone(),
                value: scenario.weight,
            }
        })?;
        validate_positive_finite(scenario.intensity, || {
            IdeaPortfolioError::InvalidScenarioIntensity {
                scenario_id: scenario.id.clone(),
                value: scenario.intensity,
            }
        })?;
    }

    for evidence in &request.evidence {
        if let Some(idea_id) = &evidence.idea_id {
            if !idea_ids.contains(idea_id) {
                return Err(IdeaPortfolioError::UnknownEvidenceIdea(idea_id.clone()));
            }
        }
        if !evidence.value.is_finite() || !(0.0..=1.0).contains(&evidence.value) {
            return Err(IdeaPortfolioError::InvalidEvidenceValue {
                metric: evidence.metric,
                value: evidence.value,
            });
        }
    }

    Ok(())
}

fn validate_trait_values(
    segment_id: &str,
    traits: &BTreeMap<String, f64>,
) -> Result<(), IdeaPortfolioError> {
    for (trait_name, value) in traits {
        validate_non_empty(
            trait_name,
            IdeaPortfolioError::InvalidSegmentTraitName(segment_id.into()),
        )?;
        if !value.is_finite() || !(0.0..=1.0).contains(value) {
            return Err(IdeaPortfolioError::InvalidSegmentTraitValue {
                segment_id: segment_id.into(),
                trait_name: trait_name.clone(),
                value: *value,
            });
        }
    }
    Ok(())
}

fn validate_non_empty(value: &str, error: IdeaPortfolioError) -> Result<(), IdeaPortfolioError> {
    if value.trim().is_empty() {
        Err(error)
    } else {
        Ok(())
    }
}

fn validate_positive_finite(
    value: f64,
    error: impl FnOnce() -> IdeaPortfolioError,
) -> Result<(), IdeaPortfolioError> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(error())
    }
}

fn segment_counts(
    segments: &[IdeaAudienceSegment],
    target_count: usize,
) -> BTreeMap<String, usize> {
    let total_weight = segments
        .iter()
        .map(|segment| segment.share_weight.max(0.0))
        .sum::<f64>()
        .max(f64::EPSILON);
    let mut remaining = target_count;
    let mut counts = BTreeMap::new();
    for (index, segment) in segments.iter().enumerate() {
        let count = if index == segments.len() - 1 {
            remaining
        } else {
            ((segment.share_weight / total_weight) * target_count as f64)
                .round()
                .max(1.0) as usize
        }
        .min(remaining);
        remaining = remaining.saturating_sub(count);
        counts.insert(segment.id.clone(), count);
    }
    counts
}

fn all_metrics() -> Vec<IdeaMetricKind> {
    vec![
        IdeaMetricKind::MarketPull,
        IdeaMetricKind::ViralLoop,
        IdeaMetricKind::BuildSpeed,
        IdeaMetricKind::AiUnlock,
        IdeaMetricKind::ConsumerClarity,
        IdeaMetricKind::WillingnessToPay,
        IdeaMetricKind::RetentionFit,
        IdeaMetricKind::DistributionFit,
        IdeaMetricKind::FounderFit,
        IdeaMetricKind::RiskResilience,
    ]
}

fn adoption_probability(
    score: f64,
    idea: &BusinessIdea,
    scenario: &IdeaScenario,
    segment: &IdeaAudienceSegment,
) -> f64 {
    clamp01(
        score
            * (0.38
                + (segment_fit(idea, segment) * 0.20)
                + (channel_fit(&idea.channels, &segment.channels, &scenario.channels) * 0.18)
                + (trait_value(segment, "trend_adoption") * 0.16)),
    )
}

fn evidence_score(
    idea: &BusinessIdea,
    metric: IdeaMetricKind,
    evidence: &[IdeaEvidence],
) -> Option<f64> {
    let matching = evidence
        .iter()
        .filter(|item| item.metric == metric)
        .filter(|item| {
            item.idea_id
                .as_deref()
                .map_or(true, |id| id == idea.id.as_str())
        })
        .collect::<Vec<_>>();
    if matching.is_empty() {
        return None;
    }
    let mut weighted = 0.0;
    let mut total = 0.0;
    for item in matching {
        let weight = item
            .sample_size
            .map(|value| value.max(1) as f64)
            .unwrap_or(1.0);
        weighted += item.value * weight;
        total += weight;
    }
    Some(clamp01(weighted / total.max(f64::EPSILON)))
}

fn evidence_notes(idea: &BusinessIdea, evidence: &[IdeaEvidence]) -> Vec<String> {
    evidence
        .iter()
        .filter(|item| item.idea_id.as_deref() == Some(idea.id.as_str()))
        .filter_map(|item| item.note.clone())
        .take(3)
        .collect()
}

fn segment_fit(idea: &BusinessIdea, segment: &IdeaAudienceSegment) -> f64 {
    if idea.target_segments.is_empty() {
        0.58
    } else if idea.target_segments.iter().any(|id| id == &segment.id) {
        0.86
    } else {
        0.42
    }
}

fn weighted_trait_fit(
    weights: &BTreeMap<String, f64>,
    traits: &BTreeMap<String, f64>,
) -> Option<f64> {
    if weights.is_empty() {
        return None;
    }
    let mut sum = 0.0;
    let mut total = 0.0;
    for (trait_name, weight) in weights {
        let value = traits.get(trait_name).copied().unwrap_or(0.5);
        let directional = if *weight >= 0.0 { value } else { 1.0 - value };
        let weight_abs = weight.abs();
        sum += directional * weight_abs;
        total += weight_abs;
    }
    (total > f64::EPSILON).then_some(clamp01(sum / total))
}

fn channel_fit(
    idea_channels: &[String],
    segment_channels: &[String],
    scenario_channels: &[String],
) -> f64 {
    if idea_channels.is_empty() {
        return 0.50;
    }
    let segment_overlap = overlap_ratio(idea_channels, segment_channels);
    let scenario_overlap = overlap_ratio(idea_channels, scenario_channels);
    clamp01(0.20 + (segment_overlap * 0.42) + (scenario_overlap * 0.38))
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
                .any(|right_value| left_value.eq_ignore_ascii_case(right_value))
        })
        .count();
    matches as f64 / left.len() as f64
}

fn overlap_text(text: &str, values: &[String]) -> f64 {
    if values.is_empty() {
        return 0.35;
    }
    let matches = values
        .iter()
        .filter(|value| text.contains(&value.to_ascii_lowercase()))
        .count();
    (matches as f64 / values.len() as f64).max(0.25)
}

fn keyword_score(text: &str, keywords: &[&str]) -> f64 {
    if keywords.is_empty() {
        return 0.0;
    }
    let matches = keywords
        .iter()
        .filter(|keyword| text.contains(&keyword.to_ascii_lowercase()))
        .count();
    (matches as f64 / keywords.len().min(4) as f64).clamp(0.0, 1.0)
}

fn risk_penalty(idea: &BusinessIdea) -> f64 {
    let text = idea_text(idea);
    clamp01(
        (idea.risks.len().min(5) as f64 * 0.08)
            + (keyword_score(
                &text,
                &["complex", "expensive", "privacy", "safety", "regulation"],
            ) * 0.18),
    )
}

fn summary_clarity(idea: &BusinessIdea) -> f64 {
    let length = idea
        .summary
        .as_deref()
        .unwrap_or(&idea.name)
        .chars()
        .count();
    if (24..=140).contains(&length) {
        0.82
    } else if length < 24 {
        0.62
    } else {
        0.48
    }
}

fn trait_value(segment: &IdeaAudienceSegment, trait_name: &str) -> f64 {
    segment
        .traits
        .get(trait_name)
        .copied()
        .unwrap_or(0.5)
        .clamp(0.0, 1.0)
}

fn idea_text(idea: &BusinessIdea) -> String {
    idea.name
        .split_whitespace()
        .chain(
            idea.summary
                .iter()
                .flat_map(|value| value.split_whitespace()),
        )
        .chain(
            idea.strengths
                .iter()
                .flat_map(|value| value.split_whitespace()),
        )
        .chain(idea.risks.iter().flat_map(|value| value.split_whitespace()))
        .chain(
            idea.category
                .iter()
                .flat_map(|value| value.split_whitespace()),
        )
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>()
        .join(" ")
}

fn inferred_category(text: &str) -> Option<String> {
    let lowered = text.to_ascii_lowercase();
    let category = if contains_any(&lowered, &["game", "roblox", "arcade"]) {
        "games"
    } else if contains_any(&lowered, &["voice", "call", "agent", "book", "refund"]) {
        "agentic_services"
    } else if contains_any(&lowered, &["image", "video", "reel", "music", "seedance"]) {
        "creative_media"
    } else if contains_any(&lowered, &["dating", "party", "irl", "event", "friend"]) {
        "social_irl"
    } else if contains_any(&lowered, &["parent", "kid", "family", "pet"]) {
        "family"
    } else {
        "consumer_ai"
    };
    Some(category.into())
}

fn inferred_target_segments(text: &str) -> Vec<String> {
    let lowered = text.to_ascii_lowercase();
    let mut segments = Vec::new();
    if contains_any(
        &lowered,
        &["creator", "ugc", "music", "newsletter", "youtube"],
    ) {
        segments.push("creator_operators".into());
    }
    if contains_any(
        &lowered,
        &["tiktok", "instagram", "meme", "friend", "style"],
    ) {
        segments.push("gen_z_social_creators".into());
    }
    if contains_any(
        &lowered,
        &["business", "restaurant", "local", "store", "shop"],
    ) {
        segments.push("local_business_owners".into());
    }
    if contains_any(&lowered, &["parent", "kid", "family", "pet", "bedtime"]) {
        segments.push("parents_and_families".into());
    }
    if contains_any(&lowered, &["game", "roblox", "arcade", "rpg"]) {
        segments.push("gamers_and_builders".into());
    }
    if contains_any(&lowered, &["dating", "irl", "party", "event"]) {
        segments.push("dating_app_fatigued".into());
    }
    if segments.is_empty() {
        segments.push("gen_z_social_creators".into());
    }
    segments.sort();
    segments.dedup();
    segments
}

fn inferred_channels(text: &str) -> Vec<String> {
    let lowered = text.to_ascii_lowercase();
    let mut channels = Vec::new();
    for (needle, channel) in [
        ("tiktok", "tiktok"),
        ("instagram", "instagram"),
        ("ig", "instagram"),
        ("x/", "x"),
        ("twitter", "x"),
        ("youtube", "youtube"),
        ("discord", "discord"),
        ("imessage", "imessage"),
        ("sms", "sms"),
        ("event", "events"),
        ("app store", "app_store"),
        ("shop", "shopify"),
    ] {
        if lowered.contains(needle) {
            channels.push(channel.into());
        }
    }
    if channels.is_empty() {
        channels.extend(["tiktok", "instagram", "app_store"].map(str::to_string));
    }
    channels.sort();
    channels.dedup();
    channels
}

fn inferred_trait_weights(text: &str) -> BTreeMap<String, f64> {
    let lowered = text.to_ascii_lowercase();
    let mut traits = BTreeMap::new();
    if contains_any(&lowered, &["viral", "share", "friend", "meme", "creator"]) {
        traits.insert("social_sharing".into(), 1.0);
    }
    if contains_any(&lowered, &["trend", "tiktok", "instagram", "style"]) {
        traits.insert("trend_adoption".into(), 0.9);
    }
    if contains_any(&lowered, &["daily", "weekly", "cohort", "streak", "habit"]) {
        traits.insert("habit_openness".into(), 1.0);
    }
    if contains_any(
        &lowered,
        &["ai", "agent", "gpt", "voice", "image", "video", "game"],
    ) {
        traits.insert("ai_openness".into(), 1.0);
    }
    if contains_any(
        &lowered,
        &["saves", "refund", "business", "commerce", "shop"],
    ) {
        traits.insert("spending_power".into(), 0.9);
    }
    if traits.is_empty() {
        traits.insert("trend_adoption".into(), 0.7);
    }
    traits
}

fn inferred_strengths(text: &str) -> Vec<String> {
    let lowered = text.to_ascii_lowercase();
    let mut strengths = Vec::new();
    for (needle, strength) in [
        ("viral", "viral loop"),
        ("creator", "creator monetization"),
        ("game", "playable loop"),
        ("voice", "voice agent"),
        ("image", "image generation"),
        ("video", "video generation"),
        ("refund", "direct savings"),
        ("local", "local demand"),
        ("family", "emotional utility"),
    ] {
        if lowered.contains(needle) {
            strengths.push(strength.into());
        }
    }
    if strengths.is_empty() {
        strengths.push("clear consumer wedge".into());
    }
    strengths
}

fn inferred_risks(text: &str) -> Vec<String> {
    let lowered = text.to_ascii_lowercase();
    let mut risks = Vec::new();
    if contains_any(&lowered, &["novelty", "meme", "image", "video"]) {
        risks.push("novelty decay".into());
    }
    if contains_any(&lowered, &["privacy", "voice", "kid", "family"]) {
        risks.push("trust and privacy sensitivity".into());
    }
    if contains_any(&lowered, &["hardware", "ar", "metal"]) {
        risks.push("higher build complexity".into());
    }
    if risks.is_empty() {
        risks.push("needs live-market validation".into());
    }
    risks
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn stable_text_seed(scope: &str, text: &str, id: &str) -> u64 {
    let mut hasher = stable_hasher(scope);
    update_hash_str(&mut hasher, text);
    update_hash_str(&mut hasher, id);
    finish_hash_u64(hasher)
}

fn stable_noise(seed_base: u64, segment_id: &str, idea_id: &str, index: usize) -> f64 {
    let mut hasher = stable_hasher("idea_individual_noise_v1");
    update_hash_u64(&mut hasher, seed_base);
    update_hash_str(&mut hasher, segment_id);
    update_hash_str(&mut hasher, idea_id);
    update_hash_u64(&mut hasher, index as u64);
    let value = finish_hash_u64(hasher);
    (value as f64 / u64::MAX as f64).clamp(0.0, 1.0)
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

fn prompt_excerpt(value: &str, max_chars: usize) -> String {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
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

fn prompt_excerpt_owned(value: &str) -> String {
    prompt_excerpt(value, 220)
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

fn default_scenario_weight() -> f64 {
    1.0
}

fn default_scenario_intensity() -> f64 {
    1.0
}

#[derive(Debug, Error)]
pub enum IdeaPortfolioDraftError {
    #[error("idea portfolio draft prompt cannot be empty")]
    EmptyPrompt,
    #[error("no numbered or table ideas were found in the draft prompt")]
    NoIdeasFound,
    #[error("drafted portfolio is invalid: {0}")]
    Portfolio(IdeaPortfolioError),
}

#[derive(Debug, Error)]
pub enum IdeaPortfolioError {
    #[error("portfolio ID cannot be empty")]
    EmptyId,
    #[error("portfolio name cannot be empty")]
    EmptyName,
    #[error("population target_count must be greater than zero")]
    ZeroPopulation,
    #[error("at least one audience segment is required")]
    NoSegments,
    #[error("at least one idea is required")]
    NoIdeas,
    #[error("at least one scenario is required")]
    NoScenarios,
    #[error("segment ID cannot be empty")]
    EmptySegmentId,
    #[error("segment name cannot be empty")]
    EmptySegmentName,
    #[error("duplicate segment ID `{0}`")]
    DuplicateSegmentId(String),
    #[error("segment `{segment_id}` share_weight must be positive and finite, got {value}")]
    InvalidSegmentShare { segment_id: String, value: f64 },
    #[error("segment `{0}` has an empty trait name")]
    InvalidSegmentTraitName(String),
    #[error("segment `{segment_id}` trait `{trait_name}` must be in [0, 1], got {value}")]
    InvalidSegmentTraitValue {
        segment_id: String,
        trait_name: String,
        value: f64,
    },
    #[error("idea ID cannot be empty")]
    EmptyIdeaId,
    #[error("idea name cannot be empty")]
    EmptyIdeaName,
    #[error("duplicate idea ID `{0}`")]
    DuplicateIdeaId(String),
    #[error("idea `{idea_id}` targets unknown segment `{segment_id}`")]
    UnknownIdeaSegment { idea_id: String, segment_id: String },
    #[error("idea `{0}` has an empty trait weight name")]
    InvalidIdeaTraitName(String),
    #[error("idea `{idea_id}` trait weight `{trait_name}` must be finite, got {value}")]
    InvalidIdeaTraitWeight {
        idea_id: String,
        trait_name: String,
        value: f64,
    },
    #[error("scenario ID cannot be empty")]
    EmptyScenarioId,
    #[error("scenario name cannot be empty")]
    EmptyScenarioName,
    #[error("duplicate scenario ID `{0}`")]
    DuplicateScenarioId(String),
    #[error("scenario `{scenario_id}` weight must be positive and finite, got {value}")]
    InvalidScenarioWeight { scenario_id: String, value: f64 },
    #[error("scenario `{scenario_id}` intensity must be positive and finite, got {value}")]
    InvalidScenarioIntensity { scenario_id: String, value: f64 },
    #[error("evidence references unknown idea `{0}`")]
    UnknownEvidenceIdea(String),
    #[error("evidence value for metric `{metric:?}` must be in [0, 1], got {value}")]
    InvalidEvidenceValue { metric: IdeaMetricKind, value: f64 },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_draft() -> IdeaPortfolioDraftRequest {
        IdeaPortfolioDraftRequest {
            name: Some("Consumer AI Ideas".into()),
            prompt: r#"
| # | Idea | First Wedge + Viral Loop |
|---:|---|---|
| 1 | PaperGames | Vibe code mobile games with a real iOS game engine. |
| 2 | Concierge | Voice agent books restaurants and services for you. |
| 3 | Tatt | AR tattoo try-on with AI-generated flash. |
| 4 | BedtimeStory | Personalized illustrated bedtime stories for kids. |
"#
            .into(),
            population: Some(IdeaPopulationConfig {
                target_count: 1_000,
                sample_size: 6,
            }),
            ..IdeaPortfolioDraftRequest::default()
        }
    }

    #[test]
    fn draft_idea_portfolio_extracts_table_ideas() {
        let portfolio = draft_idea_portfolio(&sample_draft()).unwrap();

        assert_eq!(portfolio.ideas.len(), 4);
        assert_eq!(portfolio.ideas[0].id, "papergames");
        assert_eq!(portfolio.population.target_count, 1_000);
        assert!(!portfolio.segments.is_empty());
        assert!(!portfolio.scenarios.is_empty());
    }

    #[test]
    fn idea_portfolio_runs_and_is_deterministic() {
        let portfolio = draft_idea_portfolio(&sample_draft()).unwrap();
        let first = run_idea_portfolio(&portfolio).unwrap();
        let second = run_idea_portfolio(&portfolio).unwrap();

        assert_eq!(first.ranked_ideas.len(), 4);
        assert_eq!(first.scenario_summaries.len(), portfolio.scenarios.len());
        assert_eq!(first.sampled_people.len(), 6);
        assert_eq!(
            first.ranked_ideas[0].idea_id,
            second.ranked_ideas[0].idea_id
        );
        assert_eq!(
            first.ranked_ideas[0].overall_score,
            second.ranked_ideas[0].overall_score
        );
    }

    #[test]
    fn idea_portfolio_rejects_duplicate_idea_ids() {
        let mut portfolio = draft_idea_portfolio(&sample_draft()).unwrap();
        portfolio.ideas[1].id = portfolio.ideas[0].id.clone();

        let err = run_idea_portfolio(&portfolio).unwrap_err();
        assert!(matches!(err, IdeaPortfolioError::DuplicateIdeaId(_)));
    }

    #[test]
    fn draft_idea_portfolio_rejects_empty_prompt() {
        let err = draft_idea_portfolio(&IdeaPortfolioDraftRequest::default()).unwrap_err();

        assert!(matches!(err, IdeaPortfolioDraftError::EmptyPrompt));
    }
}
