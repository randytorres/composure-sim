use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::time::{Duration, Instant};

use composure_marketing::{
    simulate_marketing_v2, ApproachDefinition, ApproachLlmAnalysis, LlmEvaluatorTrace,
    LlmPromptEvidence, LlmResponseEvidence, LlmTokenUsage, LlmUsage, MarketingLlmAnalysis,
    MarketingLlmEvidence, MarketingLlmTrace, MarketingSimulationRequestV2,
    MarketingSimulationResultV2, MetricKind, PersonaApproachResult, PrimaryScorecard,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;

const DEFAULT_CLIPROXYAPI_BASE_URL: &str = "http://127.0.0.1:8317/v1";
const DEFAULT_OPENAI_BASE_URL: &str = "https://api.openai.com/v1";
const DEFAULT_LLM_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_LLM_READ_TIMEOUT: Duration = Duration::from_secs(180);
const DEFAULT_LLM_WRITE_TIMEOUT: Duration = Duration::from_secs(30);

const DEFAULT_SYSTEM_PROMPT: &str = "You are a frontier GTM simulation analyst. Use the deterministic simulation as the anchor, not as something to ignore. Preserve numeric outputs, avoid fake certainty, call out realism gaps, and recommend experiments that could improve the model with real data. When calibration evidence exists, explain where the deterministic model is overpredicting, underpredicting, or tracking closely before offering advice.";

const PROMPT_PREAMBLE: &str = "Analyze the marketing simulation result below. Return JSON only. Do not include markdown fences. Use the deterministic scores as fixed evidence, not as something to overwrite. If observed outcomes exist, treat them as the strongest evidence source and explicitly reconcile your take with them. Read the calibration summary, observed_outcome_summary, and each approach's calibration_evidence before making recommendations. Separate absolute calibration from relative rank ordering: an approach can still be the best current bet while the deterministic layer is miscalibrated in absolute terms.";

const PROMPT_SCHEMA: &str = r#"Return exactly this JSON shape:
{
  "executive_summary": ["..."],
  "strategic_takeaways": [
    "Parameter update: ...",
    "Scenario split: ...",
    "Evidence-backed insight: ..."
  ],
  "recommended_next_experiments": [
    "Experiment: ..."
  ],
  "confidence_notes": [
    "State whether the point is backed by observed outcomes, deterministic metrics, or missing-data inference."
  ],
  "approach_analyses": [
    {
      "approach_id": "exact deterministic approach_id, ordered strongest real-world bet to weakest",
      "narrative": "string",
      "strongest_personas": ["persona ids only"],
      "objections_to_resolve": ["..."],
      "realism_warnings": ["..."],
      "next_experiments": ["Experiment: ..."]
    }
  ]
}"#;

#[derive(Debug, Clone, Default)]
pub(crate) struct AssistedSimulationOptions {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub reasoning_effort: Option<String>,
}

#[derive(Debug, Error)]
pub(crate) enum MarketingLlmError {
    #[error("marketing simulation error: {0}")]
    Simulation(#[from] composure_marketing::MarketingSimulationError),
    #[error(
        "llm-assisted simulation requires evaluator.model in the request or --model on the CLI"
    )]
    MissingModel,
    #[error("missing API key for provider {provider}; set {env_hint}")]
    MissingApiKey { provider: String, env_hint: String },
    #[error("unsupported provider {0}; expected one of: openai, cliproxyapi")]
    UnsupportedProvider(String),
    #[error("LLM request failed: {0}")]
    Http(String),
    #[error("LLM response did not include any output text")]
    MissingOutputText,
    #[error("LLM response was not valid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Deserialize)]
struct LlmAnalysisEnvelope {
    #[serde(default)]
    executive_summary: Vec<String>,
    #[serde(default)]
    strategic_takeaways: Vec<String>,
    #[serde(default)]
    recommended_next_experiments: Vec<String>,
    #[serde(default)]
    confidence_notes: Vec<String>,
    #[serde(default)]
    approach_analyses: Vec<ApproachAnalysisEnvelope>,
}

#[derive(Debug, Clone, Deserialize)]
struct ApproachAnalysisEnvelope {
    approach_id: String,
    narrative: String,
    #[serde(default)]
    strongest_personas: Vec<String>,
    #[serde(default)]
    objections_to_resolve: Vec<String>,
    #[serde(default)]
    realism_warnings: Vec<String>,
    #[serde(default)]
    next_experiments: Vec<String>,
}

#[derive(Debug, Serialize)]
struct PromptApproachSummary<'a> {
    approach_id: &'a str,
    angle: &'a str,
    format: &'a str,
    channels: &'a [String],
    tone: &'a str,
    target: &'a str,
    overall_score: u32,
    engagement_score: u32,
    viral_potential: u32,
    win_reasons: &'a [String],
    loss_risks: &'a [String],
    confidence_notes: &'a [String],
    calibration_notes: &'a [String],
    calibration_evidence: Option<ObservedOutcomeSummary>,
    strongest_personas: Vec<PromptPersonaSummary<'a>>,
}

#[derive(Debug, Serialize)]
struct PromptPersonaSummary<'a> {
    persona_id: &'a str,
    persona_name: &'a str,
    audience_weight: f64,
    overall_score: u32,
    top_reactions: &'a [String],
    concerns: &'a [String],
}

#[derive(Debug, Clone, Serialize)]
struct CalibrationGapSummary {
    observed_metric: String,
    observed_value: f64,
    proxy_metric: String,
    proxy_score: u32,
    proxy_rate: f64,
    delta: f64,
    direction: String,
}

#[derive(Debug, Clone, Serialize)]
struct ObservedOutcomeSummary {
    approach_id: String,
    total_samples: u32,
    observed_record_count: usize,
    sources: Vec<String>,
    creative_ids: Vec<String>,
    hook_ids: Vec<String>,
    landing_variants: Vec<String>,
    persona_ids: Vec<String>,
    waitlist_signup_rate: Option<f64>,
    activation_rate: Option<f64>,
    retention_d7: Option<f64>,
    paid_conversion_rate: Option<f64>,
    share_rate: Option<f64>,
    deterministic_gaps: Vec<CalibrationGapSummary>,
    scenario_split_signals: Vec<String>,
    parameter_update_candidates: Vec<String>,
    evidence_limitations: Vec<String>,
}

#[derive(Debug, Clone)]
struct EffectiveEvaluatorConfig {
    provider: String,
    model: String,
    reasoning_effort: Option<String>,
    max_output_tokens: Option<u32>,
    analysis_goal: Option<String>,
    system_prompt: String,
    evaluator_count: usize,
}

#[derive(Debug)]
struct ParsedLlmResponse {
    envelope: LlmAnalysisEnvelope,
    parsed_output: Value,
    raw_output_text: String,
}

#[derive(Debug)]
struct LlmCallRecord {
    base_url: String,
    response: Value,
    raw_stream_text: Option<String>,
    stream_fallback_used: bool,
    duration_ms: u64,
    response_id: Option<String>,
    status: Option<String>,
    usage: Option<LlmUsage>,
}

#[derive(Debug)]
struct CompletedEvaluation {
    envelope: LlmAnalysisEnvelope,
    trace: LlmEvaluatorTrace,
    evidence: LlmResponseEvidence,
}

#[derive(Debug)]
struct StreamingCallRecord {
    response_json: Option<Value>,
    output_text: String,
    raw_stream_text: String,
}

pub(crate) fn simulate_marketing_v2_assisted(
    request: &MarketingSimulationRequestV2,
) -> Result<MarketingSimulationResultV2, MarketingLlmError> {
    simulate_marketing_v2_assisted_with_options(request, &AssistedSimulationOptions::default())
}

pub(crate) fn simulate_marketing_v2_assisted_with_options(
    request: &MarketingSimulationRequestV2,
    options: &AssistedSimulationOptions,
) -> Result<MarketingSimulationResultV2, MarketingLlmError> {
    let mut result = simulate_marketing_v2(request)?;
    if !llm_assist_enabled(request) {
        return Ok(result);
    }

    let config = resolve_effective_config(request, options)?;
    let prompt_result = if request.output.include_metric_breakdown {
        result.clone()
    } else {
        let mut prompt_request = request.clone();
        prompt_request.output.include_metric_breakdown = true;
        simulate_marketing_v2(&prompt_request)?
    };
    let base_user_prompt = build_user_prompt(request, &prompt_result, &config.analysis_goal);
    let prompt_char_count = config.system_prompt.len() + base_user_prompt.len();
    let mut completed = Vec::with_capacity(config.evaluator_count);

    for evaluator_index in 0..config.evaluator_count {
        let user_prompt = build_independent_evaluator_prompt(
            &base_user_prompt,
            evaluator_index,
            config.evaluator_count,
        );
        let call = call_llm(
            &config.provider,
            &config.model,
            config.reasoning_effort.as_deref(),
            config.max_output_tokens,
            &config.system_prompt,
            &user_prompt,
        )?;
        let parsed = parse_llm_response(&call.response)?;
        completed.push(CompletedEvaluation {
            envelope: parsed.envelope,
            trace: LlmEvaluatorTrace {
                evaluator_index: evaluator_index + 1,
                provider: Some(config.provider.clone()),
                model: config.model.clone(),
                reasoning_effort: config.reasoning_effort.clone(),
                base_url: call.base_url.clone(),
                requested_max_output_tokens: config.max_output_tokens,
                stream_fallback_used: call.stream_fallback_used,
                duration_ms: call.duration_ms,
                response_id: call.response_id.clone(),
                usage: call.usage.clone(),
                raw_response: call.response.clone(),
                raw_output_text: parsed.raw_output_text.clone(),
                parsed_output: Some(parsed.parsed_output.clone()),
            },
            evidence: LlmResponseEvidence {
                base_url: Some(call.base_url),
                response_id: call.response_id,
                status: call.status,
                duration_ms: Some(call.duration_ms),
                raw_output_text: Some(parsed.raw_output_text),
                parsed_response_json: Some(parsed.parsed_output),
                raw_response_json: Some(call.response),
                raw_stream_text: call.raw_stream_text,
                usage: call.usage.as_ref().map(|usage| LlmTokenUsage {
                    input_tokens: usage.input_tokens.map(u64::from),
                    output_tokens: usage.output_tokens.map(u64::from),
                    reasoning_tokens: usage.reasoning_tokens.map(u64::from),
                    total_tokens: usage.total_tokens.map(u64::from),
                }),
                streamed_fallback_used: call.stream_fallback_used,
            },
        });
    }

    let mut top_level_analysis = aggregate_top_level_analysis(&config, &completed);
    top_level_analysis.evidence = Some(MarketingLlmEvidence {
        prompt: Some(LlmPromptEvidence {
            system_prompt: config.system_prompt.clone(),
            user_prompt: base_user_prompt.clone(),
        }),
        calls: completed
            .iter()
            .map(|evaluation| evaluation.evidence.clone())
            .collect(),
    });

    let per_approach_analysis =
        aggregate_approach_analyses(&request.approaches, config.evaluator_count, &completed);
    let llm_trace = MarketingLlmTrace {
        analysis_goal: config.analysis_goal.clone(),
        system_prompt: config.system_prompt.clone(),
        user_prompt: base_user_prompt,
        prompt_char_count,
        evaluators: completed
            .iter()
            .map(|evaluation| evaluation.trace.clone())
            .collect(),
    };

    result.engine.provider = Some(config.provider.clone());
    result.engine.model = config.model.clone();
    result.engine.reasoning_effort = config.reasoning_effort.clone();

    let mut merged_recommendations = result.recommended_next_experiments.clone();
    for item in &top_level_analysis.recommended_next_experiments {
        if !merged_recommendations.contains(item) {
            merged_recommendations.push(item.clone());
        }
    }

    for approach in &mut result.approach_results {
        approach.llm_analysis = per_approach_analysis
            .iter()
            .find(|item| item.0 == approach.approach_id)
            .map(|item| item.1.clone());
    }

    result.recommended_next_experiments = merged_recommendations;
    result.llm_analysis = Some(top_level_analysis);
    result.llm_trace = Some(llm_trace);

    Ok(result)
}

fn llm_assist_enabled(request: &MarketingSimulationRequestV2) -> bool {
    request
        .llm_assist
        .as_ref()
        .map(|config| config.enabled)
        .unwrap_or(true)
}

fn resolve_effective_config(
    request: &MarketingSimulationRequestV2,
    options: &AssistedSimulationOptions,
) -> Result<EffectiveEvaluatorConfig, MarketingLlmError> {
    let provider = options
        .provider
        .clone()
        .or_else(|| {
            request
                .evaluator
                .as_ref()
                .and_then(|config| config.provider.clone())
        })
        .unwrap_or_else(|| "openai".into());
    validate_provider(&provider)?;
    let model = options
        .model
        .clone()
        .or_else(|| {
            request
                .evaluator
                .as_ref()
                .and_then(|config| config.model.clone())
        })
        .ok_or(MarketingLlmError::MissingModel)?;
    let reasoning_effort = options.reasoning_effort.clone().or_else(|| {
        request
            .evaluator
            .as_ref()
            .and_then(|config| config.reasoning_effort.clone())
    });
    let llm_assist = request.llm_assist.as_ref();

    Ok(EffectiveEvaluatorConfig {
        provider,
        model,
        reasoning_effort,
        max_output_tokens: llm_assist.and_then(|config| config.max_output_tokens),
        analysis_goal: llm_assist.and_then(|config| config.analysis_goal.clone()),
        system_prompt: llm_assist
            .and_then(|config| config.system_prompt.clone())
            .unwrap_or_else(|| DEFAULT_SYSTEM_PROMPT.into()),
        evaluator_count: llm_assist
            .and_then(|config| config.evaluator_count)
            .filter(|count| *count > 0)
            .unwrap_or(1),
    })
}

fn aggregate_top_level_analysis(
    config: &EffectiveEvaluatorConfig,
    completed: &[CompletedEvaluation],
) -> MarketingLlmAnalysis {
    let evaluators = completed.len();
    let threshold = majority_threshold(evaluators);

    MarketingLlmAnalysis {
        provider: Some(config.provider.clone()),
        model: config.model.clone(),
        reasoning_effort: config.reasoning_effort.clone(),
        evaluator_count: evaluators,
        executive_summary: top_ranked_lines(
            completed
                .iter()
                .flat_map(|evaluation| evaluation.envelope.executive_summary.iter()),
            4,
        ),
        consensus_summary: if evaluators > 1 {
            build_top_level_consensus(completed, threshold)
        } else {
            Vec::new()
        },
        strategic_takeaways: top_ranked_lines(
            completed
                .iter()
                .flat_map(|evaluation| evaluation.envelope.strategic_takeaways.iter()),
            6,
        ),
        recommended_next_experiments: top_ranked_lines(
            completed
                .iter()
                .flat_map(|evaluation| evaluation.envelope.recommended_next_experiments.iter()),
            6,
        ),
        confidence_notes: top_ranked_lines(
            completed
                .iter()
                .flat_map(|evaluation| evaluation.envelope.confidence_notes.iter()),
            5,
        ),
        disagreement_notes: if evaluators > 1 {
            build_top_level_disagreement(completed, threshold)
        } else {
            Vec::new()
        },
        evidence: None,
    }
}

fn aggregate_approach_analyses(
    approaches: &[ApproachDefinition],
    evaluator_count: usize,
    completed: &[CompletedEvaluation],
) -> Vec<(String, ApproachLlmAnalysis)> {
    let threshold = majority_threshold(evaluator_count);

    approaches
        .iter()
        .filter_map(|approach| {
            let analyses = completed
                .iter()
                .flat_map(|evaluation| evaluation.envelope.approach_analyses.iter())
                .filter(|analysis| analysis.approach_id == approach.id)
                .cloned()
                .collect::<Vec<_>>();
            if analyses.is_empty() {
                return None;
            }

            let strongest_personas = top_ranked_lines(
                analyses
                    .iter()
                    .flat_map(|item| item.strongest_personas.iter()),
                4,
            );
            let objections_to_resolve = top_ranked_lines(
                analyses
                    .iter()
                    .flat_map(|item| item.objections_to_resolve.iter()),
                4,
            );
            let realism_warnings = top_ranked_lines(
                analyses
                    .iter()
                    .flat_map(|item| item.realism_warnings.iter()),
                4,
            );
            let next_experiments = top_ranked_lines(
                analyses
                    .iter()
                    .flat_map(|item| item.next_experiments.iter()),
                4,
            );

            let narrative =
                top_ranked_lines(analyses.iter().map(|item| item.narrative.as_str()), 1)
                    .into_iter()
                    .next()
                    .unwrap_or_else(|| approach.angle.clone());

            let mut consensus_summary = Vec::new();
            if evaluator_count > 1 {
                let repeated_personas = repeated_ranked_lines(
                    analyses
                        .iter()
                        .flat_map(|item| item.strongest_personas.iter()),
                    threshold,
                    3,
                );
                if !repeated_personas.is_empty() {
                    consensus_summary.push(format!(
                        "Repeated strongest-persona signal: {}.",
                        repeated_personas.join(", ")
                    ));
                }

                let repeated_objections = repeated_ranked_lines(
                    analyses
                        .iter()
                        .flat_map(|item| item.objections_to_resolve.iter()),
                    threshold,
                    3,
                );
                if !repeated_objections.is_empty() {
                    consensus_summary.push(format!(
                        "Common objections to resolve: {}.",
                        repeated_objections.join(", ")
                    ));
                }

                let repeated_warnings = repeated_ranked_lines(
                    analyses
                        .iter()
                        .flat_map(|item| item.realism_warnings.iter()),
                    threshold,
                    2,
                );
                if !repeated_warnings.is_empty() {
                    consensus_summary.push(format!(
                        "Repeated realism concern: {}.",
                        repeated_warnings.join("; ")
                    ));
                }
            }

            let disagreement_notes = if evaluator_count > 1 {
                build_approach_disagreement_notes(&analyses, threshold)
            } else {
                Vec::new()
            };

            Some((
                approach.id.clone(),
                ApproachLlmAnalysis {
                    narrative,
                    consensus_summary,
                    strongest_personas,
                    objections_to_resolve,
                    realism_warnings,
                    next_experiments,
                    disagreement_notes,
                },
            ))
        })
        .collect()
}

fn build_top_level_consensus(completed: &[CompletedEvaluation], threshold: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let repeated_takeaways = repeated_ranked_lines(
        completed
            .iter()
            .flat_map(|evaluation| evaluation.envelope.strategic_takeaways.iter()),
        threshold,
        3,
    );
    for takeaway in repeated_takeaways {
        lines.push(format!("Repeated across evaluators: {takeaway}"));
    }

    let repeated_experiments = repeated_ranked_lines(
        completed
            .iter()
            .flat_map(|evaluation| evaluation.envelope.recommended_next_experiments.iter()),
        threshold,
        3,
    );
    for experiment in repeated_experiments {
        lines.push(format!("Common next experiment: {experiment}"));
    }

    lines
}

fn build_top_level_disagreement(
    completed: &[CompletedEvaluation],
    threshold: usize,
) -> Vec<String> {
    let mut lines = Vec::new();
    let divergent_takeaways = minority_ranked_lines(
        completed
            .iter()
            .flat_map(|evaluation| evaluation.envelope.strategic_takeaways.iter()),
        threshold,
        3,
    );
    for takeaway in divergent_takeaways {
        lines.push(format!(
            "Evaluator disagreement surfaced around: {takeaway}"
        ));
    }

    let divergent_experiments = minority_ranked_lines(
        completed
            .iter()
            .flat_map(|evaluation| evaluation.envelope.recommended_next_experiments.iter()),
        threshold,
        3,
    );
    for experiment in divergent_experiments {
        lines.push(format!("Next-step disagreement: {experiment}"));
    }

    lines
}

fn build_approach_disagreement_notes(
    analyses: &[ApproachAnalysisEnvelope],
    threshold: usize,
) -> Vec<String> {
    let mut notes = Vec::new();
    let divergent_personas = minority_ranked_lines(
        analyses
            .iter()
            .flat_map(|item| item.strongest_personas.iter()),
        threshold,
        3,
    );
    if !divergent_personas.is_empty() {
        notes.push(format!(
            "Evaluator disagreement on strongest persona: {}.",
            divergent_personas.join(", ")
        ));
    }

    let divergent_warnings = minority_ranked_lines(
        analyses
            .iter()
            .flat_map(|item| item.realism_warnings.iter()),
        threshold,
        3,
    );
    if !divergent_warnings.is_empty() {
        notes.push(format!(
            "Evaluator disagreement on realism risks: {}.",
            divergent_warnings.join("; ")
        ));
    }

    notes
}

fn top_ranked_lines<T>(items: impl IntoIterator<Item = T>, limit: usize) -> Vec<String>
where
    T: AsRef<str>,
{
    rank_texts(items)
        .into_iter()
        .take(limit)
        .map(|item| item.0)
        .collect()
}

fn repeated_ranked_lines<T>(
    items: impl IntoIterator<Item = T>,
    threshold: usize,
    limit: usize,
) -> Vec<String>
where
    T: AsRef<str>,
{
    rank_texts(items)
        .into_iter()
        .filter(|item| item.1 >= threshold)
        .take(limit)
        .map(|item| item.0)
        .collect()
}

fn minority_ranked_lines<T>(
    items: impl IntoIterator<Item = T>,
    threshold: usize,
    limit: usize,
) -> Vec<String>
where
    T: AsRef<str>,
{
    rank_texts(items)
        .into_iter()
        .filter(|item| item.1 < threshold)
        .take(limit)
        .map(|item| item.0)
        .collect()
}

fn rank_texts<T>(items: impl IntoIterator<Item = T>) -> Vec<(String, usize)>
where
    T: AsRef<str>,
{
    let mut counts = HashMap::<String, (String, usize)>::new();

    for item in items {
        let text = item.as_ref().trim();
        if text.is_empty() {
            continue;
        }

        let normalized = normalize_text(text);
        let entry = counts
            .entry(normalized)
            .or_insert_with(|| (text.to_string(), 0));
        entry.1 += 1;
    }

    let mut ranked = counts.into_values().collect::<Vec<_>>();
    ranked.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    ranked
}

fn normalize_text(text: &str) -> String {
    text.trim()
        .trim_end_matches(['.', '!', '?'])
        .to_ascii_lowercase()
}

fn majority_threshold(evaluator_count: usize) -> usize {
    if evaluator_count <= 1 {
        1
    } else {
        (evaluator_count / 2) + 1
    }
}

fn call_llm(
    provider: &str,
    model: &str,
    reasoning_effort: Option<&str>,
    max_output_tokens: Option<u32>,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<LlmCallRecord, MarketingLlmError> {
    let base_url = resolve_base_url(provider);
    let api_key = resolve_api_key(provider)?;
    let payload = json!({
        "model": model,
        "input": [
            {
                "role": "system",
                "content": [
                    {
                        "type": "input_text",
                        "text": system_prompt
                    }
                ]
            },
            {
                "role": "user",
                "content": [
                    {
                        "type": "input_text",
                        "text": user_prompt
                    }
                ]
            }
        ]
    });

    let url = format!("{}/responses", base_url.trim_end_matches('/'));
    let started = Instant::now();

    if prefers_streaming(provider) {
        let streamed = call_llm_streaming(
            &url,
            &api_key,
            enrich_payload(payload, reasoning_effort, max_output_tokens, true),
        )?;
        let parsed = synthesize_response_json(streamed.response_json, &streamed.output_text);
        extract_output_text(&parsed).ok_or(MarketingLlmError::MissingOutputText)?;
        return Ok(LlmCallRecord {
            base_url,
            response_id: parsed
                .get("id")
                .and_then(Value::as_str)
                .map(|value| value.to_string()),
            status: parsed
                .get("status")
                .and_then(Value::as_str)
                .map(|value| value.to_string()),
            usage: extract_usage(&parsed),
            response: parsed,
            raw_stream_text: Some(streamed.raw_stream_text),
            stream_fallback_used: true,
            duration_ms: started.elapsed().as_millis() as u64,
        });
    }

    let payload = enrich_payload(payload, reasoning_effort, max_output_tokens, false);
    let response = http_agent()
        .post(&url)
        .set("Authorization", &format!("Bearer {api_key}"))
        .set("Content-Type", "application/json")
        .send_json(payload.clone());

    match response {
        Ok(response) => {
            let mut parsed = response
                .into_json::<Value>()
                .map_err(|err| MarketingLlmError::Http(err.to_string()))?;
            let mut raw_stream_text = None;
            let mut stream_fallback_used = false;

            if extract_output_text(&parsed).is_none() {
                let streamed = call_llm_streaming(
                    &url,
                    &api_key,
                    enrich_payload(payload, reasoning_effort, max_output_tokens, true),
                )?;
                raw_stream_text = Some(streamed.raw_stream_text);
                parsed = synthesize_response_json(streamed.response_json, &streamed.output_text);
                stream_fallback_used = true;
            }

            extract_output_text(&parsed).ok_or(MarketingLlmError::MissingOutputText)?;
            Ok(LlmCallRecord {
                base_url,
                response_id: parsed
                    .get("id")
                    .and_then(Value::as_str)
                    .map(|value| value.to_string()),
                status: parsed
                    .get("status")
                    .and_then(Value::as_str)
                    .map(|value| value.to_string()),
                usage: extract_usage(&parsed),
                response: parsed,
                raw_stream_text,
                stream_fallback_used,
                duration_ms: started.elapsed().as_millis() as u64,
            })
        }
        Err(ureq::Error::Status(code, response)) => {
            let body = response
                .into_string()
                .unwrap_or_else(|_| "<failed to read body>".into());
            Err(MarketingLlmError::Http(format!(
                "status {code} from {url}: {body}"
            )))
        }
        Err(err) => Err(MarketingLlmError::Http(err.to_string())),
    }
}

fn enrich_payload(
    mut payload: Value,
    reasoning_effort: Option<&str>,
    max_output_tokens: Option<u32>,
    stream: bool,
) -> Value {
    if let Some(effort) = reasoning_effort {
        payload["reasoning"] = json!({ "effort": effort });
    }
    if let Some(limit) = max_output_tokens {
        payload["max_output_tokens"] = json!(limit);
    }
    if stream {
        payload["stream"] = json!(true);
    }
    payload
}

fn call_llm_streaming(
    url: &str,
    api_key: &str,
    payload: Value,
) -> Result<StreamingCallRecord, MarketingLlmError> {
    let response = http_agent()
        .post(url)
        .set("Authorization", &format!("Bearer {api_key}"))
        .set("Content-Type", "application/json")
        .send_json(payload);

    match response {
        Ok(response) => parse_streaming_response_reader(response.into_reader()),
        Err(ureq::Error::Status(code, response)) => {
            let body = response
                .into_string()
                .unwrap_or_else(|_| "<failed to read body>".into());
            return Err(MarketingLlmError::Http(format!(
                "status {code} from {url}: {body}"
            )));
        }
        Err(err) => return Err(MarketingLlmError::Http(err.to_string())),
    }
}

fn synthesize_response_json(response_json: Option<Value>, output_text: &str) -> Value {
    let content = json!([
        {
            "type": "output_text",
            "text": output_text
        }
    ]);

    match response_json {
        Some(mut response) => {
            let output_is_empty = response
                .get("output")
                .and_then(Value::as_array)
                .map(|items| items.is_empty())
                .unwrap_or(true);
            if output_is_empty {
                response["output"] = json!([
                    {
                        "content": content
                    }
                ]);
            }
            response
        }
        None => json!({
            "output": [
                {
                    "content": content
                }
            ]
        }),
    }
}

#[cfg(test)]
fn parse_streaming_response_body(body: &str) -> Result<StreamingCallRecord, MarketingLlmError> {
    parse_streaming_lines(
        body.lines().map(|line| line.to_string()),
        Some(body.to_string()),
    )
}

fn parse_streaming_response_reader(
    reader: impl Read,
) -> Result<StreamingCallRecord, MarketingLlmError> {
    let mut raw_stream_text = String::new();
    let mut lines = Vec::new();

    for line in BufReader::new(reader).lines() {
        let line = line.map_err(|err| MarketingLlmError::Http(err.to_string()))?;
        raw_stream_text.push_str(&line);
        raw_stream_text.push('\n');
        let done = line.trim() == "data: [DONE]";
        lines.push(line);
        if done {
            break;
        }
    }

    parse_streaming_lines(lines, Some(raw_stream_text))
}

fn parse_streaming_lines(
    lines: impl IntoIterator<Item = String>,
    raw_stream_text: Option<String>,
) -> Result<StreamingCallRecord, MarketingLlmError> {
    let mut deltas = String::new();
    let mut done_parts = Vec::new();
    let mut response_json = None;
    for line in lines {
        let trimmed = line.trim();
        if trimmed == "data: [DONE]" {
            break;
        }
        let Some(json_payload) = line.strip_prefix("data: ") else {
            continue;
        };
        if json_payload.trim().is_empty() {
            continue;
        }
        let Ok(event) = serde_json::from_str::<Value>(json_payload) else {
            continue;
        };
        match event.get("type").and_then(Value::as_str) {
            Some("response.output_text.delta") => {
                if let Some(delta) = event.get("delta").and_then(Value::as_str) {
                    deltas.push_str(delta);
                }
            }
            Some("response.output_text.done") => {
                if let Some(text) = event.get("text").and_then(Value::as_str) {
                    done_parts.push(text.to_string());
                }
            }
            Some("response.completed") | Some("response.failed") => {
                if let Some(response) = event.get("response") {
                    response_json = Some(response.clone());
                }
            }
            _ => {}
        }
    }

    let output_text = if !done_parts.is_empty() {
        done_parts.join("\n")
    } else {
        deltas
    };
    let output_text = if output_text.trim().is_empty() {
        response_json
            .as_ref()
            .and_then(extract_output_text)
            .unwrap_or_default()
    } else {
        output_text
    };

    if output_text.trim().is_empty() {
        return Err(MarketingLlmError::MissingOutputText);
    }

    Ok(StreamingCallRecord {
        response_json,
        output_text,
        raw_stream_text: raw_stream_text.unwrap_or_default(),
    })
}

fn http_agent() -> ureq::Agent {
    ureq::builder()
        .timeout_connect(DEFAULT_LLM_CONNECT_TIMEOUT)
        .timeout_read(DEFAULT_LLM_READ_TIMEOUT)
        .timeout_write(DEFAULT_LLM_WRITE_TIMEOUT)
        .build()
}

fn validate_provider(provider: &str) -> Result<(), MarketingLlmError> {
    match provider {
        "openai" | "cliproxyapi" => Ok(()),
        _ => Err(MarketingLlmError::UnsupportedProvider(provider.into())),
    }
}

fn prefers_streaming(provider: &str) -> bool {
    matches!(provider, "cliproxyapi")
}

fn resolve_base_url(provider: &str) -> String {
    match provider {
        "cliproxyapi" => std::env::var("CLIPROXYAPI_BASE_URL")
            .or_else(|_| std::env::var("OPENAI_BASE_URL"))
            .unwrap_or_else(|_| DEFAULT_CLIPROXYAPI_BASE_URL.into()),
        _ => std::env::var("OPENAI_BASE_URL").unwrap_or_else(|_| DEFAULT_OPENAI_BASE_URL.into()),
    }
}

fn resolve_api_key(provider: &str) -> Result<String, MarketingLlmError> {
    match provider {
        "cliproxyapi" => std::env::var("CLIPROXYAPI_API_KEY")
            .or_else(|_| std::env::var("OPENAI_API_KEY"))
            .map_err(|_| MarketingLlmError::MissingApiKey {
                provider: provider.into(),
                env_hint: "CLIPROXYAPI_API_KEY or OPENAI_API_KEY".into(),
            }),
        _ => std::env::var("OPENAI_API_KEY").map_err(|_| MarketingLlmError::MissingApiKey {
            provider: provider.into(),
            env_hint: "OPENAI_API_KEY".into(),
        }),
    }
}

fn extract_usage(response: &Value) -> Option<LlmUsage> {
    let usage = response.get("usage")?;
    let input_tokens = usage
        .get("input_tokens")
        .and_then(Value::as_u64)
        .map(|value| value as u32);
    let output_tokens = usage
        .get("output_tokens")
        .and_then(Value::as_u64)
        .map(|value| value as u32);
    let total_tokens = usage
        .get("total_tokens")
        .and_then(Value::as_u64)
        .map(|value| value as u32);
    let reasoning_tokens = usage
        .pointer("/output_tokens_details/reasoning_tokens")
        .and_then(Value::as_u64)
        .map(|value| value as u32);

    if input_tokens.is_none()
        && output_tokens.is_none()
        && total_tokens.is_none()
        && reasoning_tokens.is_none()
    {
        return None;
    }

    Some(LlmUsage {
        input_tokens,
        output_tokens,
        reasoning_tokens,
        total_tokens,
    })
}

fn parse_llm_response(response: &Value) -> Result<ParsedLlmResponse, MarketingLlmError> {
    let raw_output_text =
        extract_output_text(response).ok_or(MarketingLlmError::MissingOutputText)?;
    let json_text = extract_json_object(&raw_output_text);
    let parsed_output: Value = serde_json::from_str(&json_text)?;
    let envelope: LlmAnalysisEnvelope = serde_json::from_value(parsed_output.clone())?;

    Ok(ParsedLlmResponse {
        envelope,
        parsed_output,
        raw_output_text,
    })
}

fn extract_output_text(response: &Value) -> Option<String> {
    let mut texts = Vec::new();
    collect_output_text(response, &mut texts);
    let joined = texts
        .into_iter()
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    if joined.trim().is_empty() {
        None
    } else {
        Some(joined)
    }
}

fn collect_output_text(value: &Value, texts: &mut Vec<String>) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_output_text(item, texts);
            }
        }
        Value::Object(map) => {
            if map.get("type").and_then(Value::as_str) == Some("output_text") {
                if let Some(text) = map.get("text").and_then(Value::as_str) {
                    texts.push(text.into());
                }
            }
            if let Some(text) = map.get("output_text").and_then(Value::as_str) {
                texts.push(text.into());
            }
            for value in map.values() {
                collect_output_text(value, texts);
            }
        }
        _ => {}
    }
}

fn extract_json_object(text: &str) -> String {
    let trimmed = text.trim();
    if let Some(stripped) = strip_code_fences(trimmed) {
        return stripped;
    }
    if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}')) {
        return trimmed[start..=end].to_string();
    }
    trimmed.to_string()
}

fn strip_code_fences(text: &str) -> Option<String> {
    if !text.starts_with("```") {
        return None;
    }
    let mut lines = text.lines();
    lines.next()?;
    let mut body = Vec::new();
    for line in lines {
        if line.trim_start().starts_with("```") {
            break;
        }
        body.push(line);
    }
    Some(body.join("\n"))
}

fn build_independent_evaluator_prompt(
    base_prompt: &str,
    evaluator_index: usize,
    evaluator_count: usize,
) -> String {
    format!(
        "Independent evaluator {}/{}.\nAssess this request on your own rather than trying to hedge toward what other evaluators might say.\n\n{}",
        evaluator_index + 1,
        evaluator_count,
        base_prompt
    )
}

fn build_user_prompt(
    request: &MarketingSimulationRequestV2,
    result: &MarketingSimulationResultV2,
    analysis_goal: &Option<String>,
) -> String {
    let observed_summary = summarize_observed_outcomes(request, result);
    let approaches = request
        .approaches
        .iter()
        .map(|approach| summarize_approach(result, approach, &observed_summary))
        .collect::<Vec<_>>();
    let prompt_payload = json!({
        "analysis_goal": analysis_goal
            .clone()
            .unwrap_or_else(|| "Evaluate which approaches are most believable, commercially useful, and worth testing next in the real world.".into()),
        "project": &request.project,
        "scenario": &request.scenario,
        "personas": &request.personas,
        "channels": &request.channels,
        "audience_weighting": &request.audience_weighting,
        "observed_outcomes": &request.observed_outcomes,
        "observed_outcome_summary": observed_summary,
        "deterministic_summary": {
            "simulation_id": &result.simulation_id,
            "overall_score": result.primary_scorecard.overall_score,
            "cross_approach_insights": &result.cross_approach_insights,
            "recommended_next_experiments": &result.recommended_next_experiments,
            "calibration_summary": &result.calibration_summary,
            "approaches": approaches
        },
        "instructions": {
            "return_json_only": true,
            "do_not_replace_scores": true,
            "anchor_to_observed_outcomes_when_available": true,
            "prefer_observed_rankings_over_simulated_rankings_when_sample_sizes_are_meaningful": true,
            "call_out_where_the_deterministic_proxy_looks_over_or_under_confident": true,
            "tie_approach_narratives_to_calibration_evidence_when_present": true,
            "be_concrete_about_realism_and_failure_modes": true
        }
    });

    format!(
        "{PROMPT_PREAMBLE}\n\n{}\n\n{PROMPT_SCHEMA}",
        serde_json::to_string_pretty(&prompt_payload).unwrap_or_else(|_| "{}".into())
    )
}

fn summarize_observed_outcomes(
    request: &MarketingSimulationRequestV2,
    result: &MarketingSimulationResultV2,
) -> Vec<ObservedOutcomeSummary> {
    request
        .approaches
        .iter()
        .filter_map(|approach| {
            let observed = request
                .observed_outcomes
                .iter()
                .filter(|item| item.approach_id == approach.id)
                .collect::<Vec<_>>();
            if observed.is_empty() {
                return None;
            }

            let total_samples = observed
                .iter()
                .map(|item| item.sample_size.unwrap_or(1))
                .sum::<u32>();
            let approach_result = result
                .approach_results
                .iter()
                .find(|item| item.approach_id == approach.id)?;

            let waitlist_signup_rate = weighted_average(observed.iter().filter_map(|item| {
                item.waitlist_signup_rate
                    .map(|value| (value, item.sample_size.unwrap_or(1) as f64))
            }));
            let activation_rate = weighted_average(observed.iter().filter_map(|item| {
                item.activation_rate
                    .map(|value| (value, item.sample_size.unwrap_or(1) as f64))
            }));
            let retention_d7 = weighted_average(observed.iter().filter_map(|item| {
                item.retention_d7
                    .map(|value| (value, item.sample_size.unwrap_or(1) as f64))
            }));
            let paid_conversion_rate = weighted_average(observed.iter().filter_map(|item| {
                item.paid_conversion_rate
                    .map(|value| (value, item.sample_size.unwrap_or(1) as f64))
            }));
            let share_rate = weighted_average(observed.iter().filter_map(|item| {
                item.share_rate
                    .map(|value| (value, item.sample_size.unwrap_or(1) as f64))
            }));

            let deterministic_gaps = [
                (
                    waitlist_signup_rate,
                    MetricKind::ConversionIntent,
                    "waitlist_signup_rate",
                    "conversion_intent",
                ),
                (
                    activation_rate,
                    MetricKind::AudienceReceptivity,
                    "activation_rate",
                    "audience_receptivity",
                ),
                (
                    share_rate,
                    MetricKind::Shareability,
                    "share_rate",
                    "shareability",
                ),
                (
                    retention_d7,
                    MetricKind::RetentionFit,
                    "retention_d7",
                    "retention_fit",
                ),
            ]
            .into_iter()
            .filter_map(|(observed_value, metric, observed_metric, proxy_metric)| {
                let observed_value = observed_value?;
                let proxy_score = metric_score(&approach_result.primary_scorecard, metric)?;
                let proxy_rate = proxy_score as f64 / 100.0;
                Some(CalibrationGapSummary {
                    observed_metric: observed_metric.into(),
                    observed_value,
                    proxy_metric: proxy_metric.into(),
                    proxy_score,
                    proxy_rate,
                    delta: observed_value - proxy_rate,
                    direction: if observed_value >= proxy_rate {
                        "deterministic_underprediction".into()
                    } else {
                        "deterministic_overprediction".into()
                    },
                })
            })
            .collect::<Vec<_>>();

            Some(ObservedOutcomeSummary {
                approach_id: approach.id.clone(),
                total_samples,
                observed_record_count: observed.len(),
                sources: observed
                    .iter()
                    .filter_map(|item| item.source.clone())
                    .collect::<std::collections::BTreeSet<_>>()
                    .into_iter()
                    .collect(),
                creative_ids: observed
                    .iter()
                    .filter_map(|item| item.creative_id.clone())
                    .collect::<std::collections::BTreeSet<_>>()
                    .into_iter()
                    .collect(),
                hook_ids: observed
                    .iter()
                    .filter_map(|item| item.hook_id.clone())
                    .collect::<std::collections::BTreeSet<_>>()
                    .into_iter()
                    .collect(),
                landing_variants: observed
                    .iter()
                    .filter_map(|item| item.landing_variant.clone())
                    .collect::<std::collections::BTreeSet<_>>()
                    .into_iter()
                    .collect(),
                persona_ids: observed
                    .iter()
                    .filter_map(|item| item.persona_id.clone())
                    .collect::<std::collections::BTreeSet<_>>()
                    .into_iter()
                    .collect(),
                waitlist_signup_rate,
                activation_rate,
                retention_d7,
                paid_conversion_rate,
                share_rate,
                deterministic_gaps,
                scenario_split_signals: Vec::new(),
                parameter_update_candidates: Vec::new(),
                evidence_limitations: Vec::new(),
            })
        })
        .collect()
}

fn weighted_average(items: impl IntoIterator<Item = (f64, f64)>) -> Option<f64> {
    let mut weighted_sum = 0.0;
    let mut total_weight = 0.0;
    for (value, weight) in items {
        weighted_sum += value * weight;
        total_weight += weight;
    }
    if total_weight > 0.0 {
        Some(weighted_sum / total_weight)
    } else {
        None
    }
}

fn metric_score(scorecard: &PrimaryScorecard, metric_kind: MetricKind) -> Option<u32> {
    scorecard
        .metrics
        .iter()
        .find(|metric| metric.metric == metric_kind)
        .map(|metric| metric.score)
}

fn summarize_approach<'a>(
    result: &'a MarketingSimulationResultV2,
    approach: &'a ApproachDefinition,
    observed_summary: &[ObservedOutcomeSummary],
) -> PromptApproachSummary<'a> {
    let computed = result
        .approach_results
        .iter()
        .find(|item| item.approach_id == approach.id)
        .expect("deterministic result missing approach");
    let strongest_personas = strongest_personas(&computed.persona_results);

    PromptApproachSummary {
        approach_id: &approach.id,
        angle: &approach.angle,
        format: &approach.format,
        channels: &approach.channels,
        tone: &approach.tone,
        target: &approach.target,
        overall_score: computed.primary_scorecard.overall_score,
        engagement_score: computed.engagement_score,
        viral_potential: computed.viral_potential,
        win_reasons: &computed.win_reasons,
        loss_risks: &computed.loss_risks,
        confidence_notes: &computed.confidence_notes,
        calibration_notes: &computed.calibration_notes,
        calibration_evidence: observed_summary
            .iter()
            .find(|item| item.approach_id == approach.id)
            .cloned(),
        strongest_personas,
    }
}

fn strongest_personas(persona_results: &[PersonaApproachResult]) -> Vec<PromptPersonaSummary<'_>> {
    let mut ranked = persona_results.iter().collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .primary_scorecard
            .overall_score
            .cmp(&left.primary_scorecard.overall_score)
    });
    ranked
        .into_iter()
        .take(3)
        .map(|persona| PromptPersonaSummary {
            persona_id: &persona.persona_id,
            persona_name: &persona.persona_name,
            audience_weight: persona.audience_weight,
            overall_score: persona.primary_scorecard.overall_score,
            top_reactions: &persona.top_reactions,
            concerns: &persona.concerns,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn extracts_json_from_code_fence() {
        let raw = "```json\n{\"executive_summary\":[],\"strategic_takeaways\":[],\"recommended_next_experiments\":[],\"confidence_notes\":[],\"approach_analyses\":[]}\n```";
        let parsed: Value = serde_json::from_str(&extract_json_object(raw)).unwrap();
        assert!(parsed["executive_summary"].as_array().unwrap().is_empty());
    }

    #[test]
    fn collects_output_text_from_responses_shape() {
        let response = json!({
            "output": [
                {
                    "content": [
                        {
                            "type": "output_text",
                            "text": "{\"executive_summary\":[],\"strategic_takeaways\":[],\"recommended_next_experiments\":[],\"confidence_notes\":[],\"approach_analyses\":[]}"
                        }
                    ]
                }
            ]
        });

        let parsed = parse_llm_response(&response).unwrap();
        assert!(parsed.envelope.approach_analyses.is_empty());
    }

    #[test]
    fn ranks_repeated_lines_first() {
        let ranked = top_ranked_lines(
            [
                "Keep the hook specific",
                "Keep the hook specific",
                "Lead with proof",
            ]
            .iter()
            .copied(),
            2,
        );
        assert_eq!(ranked[0], "Keep the hook specific");
    }

    #[test]
    fn build_user_prompt_embeds_approach_level_calibration_evidence() {
        let request: MarketingSimulationRequestV2 = serde_json::from_value(json!({
            "project": {
                "name": "Composure",
                "description": "Deterministic simulation for campaigns"
            },
            "personas": [
                {
                    "id": "dev",
                    "name": "Pragmatic Dev",
                    "type": "developer"
                }
            ],
            "approaches": [
                {
                    "id": "specific",
                    "angle": "Show founders how to rank hooks before publishing",
                    "format": "Twitter thread",
                    "channels": ["twitter"],
                    "tone": "direct",
                    "target": "technical founders"
                }
            ],
            "scenario": {
                "name": "Short-form hook test",
                "scenario_type": "short_form_video",
                "time_steps": 6
            },
            "observed_outcomes": [
                {
                    "approach_id": "specific",
                    "waitlist_signup_rate": 0.19,
                    "activation_rate": 0.34,
                    "retention_d7": 0.22,
                    "share_rate": 0.11,
                    "sample_size": 180
                }
            ],
            "simulation_size": 8
        }))
        .unwrap();

        let result = simulate_marketing_v2(&request).unwrap();
        let prompt = build_user_prompt(&request, &result, &None);

        assert!(prompt.contains("\"calibration_evidence\""));
        assert!(prompt.contains("\"deterministic_gaps\""));
        assert!(
            prompt.contains("deterministic_overprediction")
                || prompt.contains("deterministic_underprediction")
        );
    }

    #[test]
    fn build_user_prompt_keeps_deterministic_gaps_when_metric_breakdowns_are_hidden() {
        let mut request: MarketingSimulationRequestV2 = serde_json::from_value(json!({
            "project": {
                "name": "Composure",
                "description": "Deterministic simulation for campaigns"
            },
            "personas": [
                {
                    "id": "dev",
                    "name": "Pragmatic Dev",
                    "type": "developer"
                }
            ],
            "approaches": [
                {
                    "id": "specific",
                    "angle": "Show founders how to rank hooks before publishing",
                    "format": "Twitter thread",
                    "channels": ["twitter"],
                    "tone": "direct",
                    "target": "technical founders"
                }
            ],
            "scenario": {
                "name": "Short-form hook test",
                "scenario_type": "short_form_video",
                "time_steps": 6
            },
            "observed_outcomes": [
                {
                    "approach_id": "specific",
                    "waitlist_signup_rate": 0.19,
                    "activation_rate": 0.34,
                    "retention_d7": 0.22,
                    "share_rate": 0.11,
                    "sample_size": 180
                }
            ],
            "simulation_size": 8
        }))
        .unwrap();
        request.output.include_metric_breakdown = false;

        let prompt_request = {
            let mut cloned = request.clone();
            cloned.output.include_metric_breakdown = true;
            cloned
        };
        let result = simulate_marketing_v2(&prompt_request).unwrap();
        let prompt = build_user_prompt(&request, &result, &None);

        assert!(prompt.contains("\"deterministic_gaps\""));
        assert!(prompt.contains("\"conversion_intent\""));
    }

    #[test]
    fn parse_streaming_response_uses_completed_response_json_when_needed() {
        let body = concat!(
            "data: {\"type\":\"response.completed\",\"response\":{\"output\":[{\"content\":[{\"type\":\"output_text\",\"text\":\"{\\\"executive_summary\\\":[],\\\"strategic_takeaways\\\":[],\\\"recommended_next_experiments\\\":[],\\\"confidence_notes\\\":[],\\\"approach_analyses\\\":[]}\"}]}]}}\n",
            "data: [DONE]\n"
        );

        let parsed = parse_streaming_response_body(body).unwrap();
        assert!(parsed.output_text.contains("\"executive_summary\""));
        assert!(parsed.response_json.is_some());
    }

    #[test]
    fn parse_streaming_response_reader_stops_at_done() {
        let body = concat!(
            "data: {\"type\":\"response.output_text.delta\",\"delta\":\"{\\\"executive_summary\\\":[]\"}\n",
            "data: {\"type\":\"response.output_text.delta\",\"delta\":\",\\\"strategic_takeaways\\\":[],\\\"recommended_next_experiments\\\":[],\\\"confidence_notes\\\":[],\\\"approach_analyses\\\":[]}\"}\n",
            "data: [DONE]\n",
            "data: {\"type\":\"response.output_text.delta\",\"delta\":\"ignored\"}\n"
        );

        let parsed = parse_streaming_response_reader(Cursor::new(body)).unwrap();
        assert!(parsed.output_text.contains("\"strategic_takeaways\""));
        assert!(!parsed.output_text.contains("ignored"));
    }

    #[test]
    fn resolve_effective_config_rejects_unsupported_provider() {
        let request: MarketingSimulationRequestV2 = serde_json::from_value(json!({
            "project": {
                "name": "Composure",
                "description": "Deterministic simulation for campaigns"
            },
            "approaches": [{
                "id": "specific",
                "angle": "Show proof",
                "format": "landing page headline",
                "channels": ["landing page"],
                "tone": "clear",
                "target": "founders"
            }],
            "evaluator": {
                "provider": "openia",
                "model": "gpt-5.4"
            },
            "simulation_size": 8
        }))
        .unwrap();

        let err =
            resolve_effective_config(&request, &AssistedSimulationOptions::default()).unwrap_err();
        assert!(
            matches!(err, MarketingLlmError::UnsupportedProvider(provider) if provider == "openia")
        );
    }
}
