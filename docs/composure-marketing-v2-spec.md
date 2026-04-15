# composure-marketing v2 spec

## Purpose

Define the next version of the reusable marketing adapter in `composure-sim`.

This spec is designed to bridge from the current implementation to a much more
useful system without throwing away what already works.

Document status:

- The sections through `Current implementation status` describe behavior that is
  already present in the repo.
- The Rust type snippets near the top are meant to reflect the implemented V2
  request and assisted-run surfaces.
- Later sections still include forward-looking design material from the original
  V2 proposal and should be read as roadmap context, not as the exact current
  wire schema.
- The source of truth for the implemented schema is
  `crates/composure-marketing/src/lib.rs`.

The immediate goal is to move from:

- one blended audience
- one approach at a time
- two headline scores

to:

- persona-level results
- richer metric families
- channel-native outputs
- reusable sequence and counterfactual support

## Current v1 summary

The current adapter takes:

- `seed_data`
- `approaches`
- `simulation_size`
- `platforms`

It builds an `AudienceProfile` from text overlap and evaluates each approach in
a fixed 6-step scenario with a 3-dimension state:

- attention
- resonance
- share propensity

And it returns:

- `engagement_score`
- `viral_potential`
- sentiment
- concerns
- reactions
- behaviors
- one trajectory

This is a good starting point, but it is too compressed for serious planning.

## V2 design goals

## Current implementation status

The first V2 slice is now scaffolded in code:

- `simulate-marketing-v2` exists in the CLI
- `simulate-marketing-v2-assisted` now exists in the CLI as an optional OpenAI-compatible enrichment layer
- persona-level scorecards and weighting exist
- scenario families now affect both metric emphasis and raw simulation dynamics
- requests can now include `evaluator`, `llm_assist`, and `observed_outcomes`

Implemented scenario families currently include:

- `audience_discovery`
- `positioning`
- `campaign_sequence`
- `community_activation`
- `retention`
- `landing_page`
- `short_form_video`
- `community_event`
- `in_store_enablement`
- `private_relationship`

This is still an intermediate step. The simulator is richer than V1, but it is
not yet a calibrated funnel model, though V2 now includes optional observed
outcome contracts so downstream repos can attach real conversion and retention
data, and an optional LLM-assisted pass can add richer judgment without moving
network access into the deterministic core crate.

### 1. Keep the adapter reusable

V2 must work for:

- consumer apps
- local community brands
- B2B products
- retail enablement scenarios
- partner or relationship marketing

### 2. Preserve deterministic execution

V2 should remain:

- seedable
- serializable
- reproducible
- artifact-first

### 3. Add explainability before complexity

Before building full network simulation, V2 should make its outputs much more
interpretable.

### 4. Stay compatible with later sequence and network work

The V2 schema should be designed so V3 can add:

- campaign sequences
- multi-entity propagation
- calibration layers

without breaking the conceptual model.

## Implemented V2 request model

## Top-level request

```rust
pub struct MarketingSimulationRequestV2 {
    pub project: ProjectContext,
    pub personas: Vec<PersonaDefinition>,
    pub approaches: Vec<ApproachDefinition>,
    #[serde(default)]
    pub channels: Vec<ChannelContext>,
    #[serde(default)]
    pub audience_weighting: Vec<AudienceWeighting>,
    #[serde(default)]
    pub scenario: ScenarioDefinition,
    #[serde(default)]
    pub evaluator: Option<EvaluatorConfig>,
    #[serde(default)]
    pub llm_assist: Option<LlmAssistConfig>,
    #[serde(default)]
    pub observed_outcomes: Vec<ObservedOutcome>,
    #[serde(default)]
    pub output: OutputOptions,
    #[serde(default = "default_simulation_size")]
    pub simulation_size: usize,
}
```

## Assisted-run configuration

These fields exist so downstream repos can keep the deterministic V2 request as
the source of truth while still attaching provider/model metadata and optional
frontier-model analysis to one run artifact.

```rust
pub struct EvaluatorConfig {
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub reasoning_effort: Option<String>,
}

pub struct LlmAssistConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub evaluator_count: Option<usize>,
    #[serde(default)]
    pub analysis_goal: Option<String>,
    #[serde(default)]
    pub max_output_tokens: Option<u32>,
    #[serde(default)]
    pub system_prompt: Option<String>,
}

pub struct ObservedOutcome {
    pub approach_id: String,
    #[serde(default)]
    pub persona_id: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub creative_id: Option<String>,
    #[serde(default)]
    pub hook_id: Option<String>,
    #[serde(default)]
    pub landing_variant: Option<String>,
    #[serde(default)]
    pub waitlist_signup_rate: Option<f64>,
    #[serde(default)]
    pub activation_rate: Option<f64>,
    #[serde(default)]
    pub retention_d7: Option<f64>,
    #[serde(default)]
    pub paid_conversion_rate: Option<f64>,
    #[serde(default)]
    pub share_rate: Option<f64>,
    #[serde(default)]
    pub sample_size: Option<u32>,
}
```

Operational notes:

- `evaluator.provider` selects the OpenAI-compatible endpoint. Today the CLI
  treats `openai` as the hosted default and `cliproxyapi` as the local proxy
  default.
- `evaluator.model` is required whenever the assisted pass is enabled. The
  assisted CLI command can now override it with `--model`.
- `evaluator.reasoning_effort` is optional metadata plus a direct pass-through
  to the Responses API `reasoning.effort` field. The assisted CLI command can
  override it with `--reasoning-effort`.
- `llm_assist.enabled = false` keeps the request on a deterministic, no-network
  path, which is useful for local artifact generation and tests.
- `observed_outcomes` gives the deterministic scorecard and assisted analysis a
  stable place to look for real signup, activation, retention, and share data.
- `simulate-marketing-v2-assisted` also accepts `--provider`, `--model`, and
  `--reasoning-effort` so a team can swap evaluator settings without editing the
  checked-in request JSON.

## Project context

```rust
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
```

Purpose:

- replace the looser `seed_data.project_*` shape
- keep project-level inputs explicit

## Persona definition

```rust
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
```

Key changes:

- add stable `id`
- keep request-level weighting separate via `audience_weighting`
- add conversion-relevant traits
- keep enough structure for deterministic heuristics

## Compare and assisted-result notes

Current behavior worth knowing before reading the older forward-looking sections:

- `simulate-marketing-v2` is still the deterministic source of truth
- `simulate-marketing-v2-assisted` enriches that artifact with LLM narrative, consensus, disagreement, and evidence traces
- `compare-marketing-v2-assisted` still ranks scenarios deterministically today
- compare-mode `metric_deltas` are currently based on each scenario's aggregate `primary_scorecard`, not the winning approach alone
- when `output.include_metric_breakdown = false`, user-facing metric arrays are hidden, but the assisted path can still rebuild the internal metric view needed for calibration evidence

## Forward-looking design material

The remaining sections below preserve earlier V2 planning ideas. They are still
useful as roadmap context, but some field names and example snippets no longer
match the exact implemented structs above.

## Approach definition

```rust
pub struct ApproachDefinition {
    pub id: String,
    pub angle: String,
    pub format: String,
    pub tone: String,
    pub target: String,
    #[serde(default)]
    pub channels: Vec<String>,
    #[serde(default)]
    pub proof_points: Vec<String>,
    #[serde(default)]
    pub objection_handlers: Vec<String>,
    #[serde(default)]
    pub cta: Option<String>,
    #[serde(default)]
    pub stage: Option<ApproachStage>,
    #[serde(default)]
    pub metadata: Option<Value>,
}
```

New value:

- `proof_points` makes "show the receipts" explicit
- `objection_handlers` captures whether the creative actually resolves concerns
- `stage` helps sequence planning

## Channel context

```rust
pub struct ChannelContext {
    pub id: String,
    pub channel_type: ChannelType,
    #[serde(default)]
    pub norms: Vec<String>,
    #[serde(default)]
    pub friction: Option<f64>,
    #[serde(default)]
    pub trust_baseline: Option<f64>,
    #[serde(default)]
    pub virality_baseline: Option<f64>,
}
```

Example channel types:

- `landing_page`
- `short_form_video`
- `social_feed`
- `stories`
- `in_store`
- `community_event`
- `private_event`
- `email`

This lets the model stop treating all channels as simple text tokens.

## Objective definition

```rust
pub struct ObjectiveDefinition {
    pub metric: MarketingMetric,
    pub weight: f64,
}
```

Examples:

- optimize for awareness
- optimize for conversion
- optimize for community formation
- optimize for recommendation lift

This is important because AC and Mirrorlife optimize for different things.

## Scenario definition

```rust
pub struct ScenarioDefinition {
    #[serde(default = "default_scenario_type")]
    pub scenario_type: ScenarioType,
    #[serde(default = "default_time_steps")]
    pub time_steps: usize,
    #[serde(default)]
    pub repeated_exposure: bool,
    #[serde(default)]
    pub fatigue_enabled: bool,
    #[serde(default)]
    pub trust_compounding_enabled: bool,
}
```

Example scenario types:

- `one_shot_message`
- `landing_page_evaluation`
- `short_form_hook`
- `community_event`
- `in_store_enablement`
- `private_relationship`
- `campaign_sequence`

## Output options

```rust
pub struct OutputOptions {
    #[serde(default)]
    pub include_persona_breakdown: bool,
    #[serde(default)]
    pub include_metric_trajectories: bool,
    #[serde(default)]
    pub include_failure_modes: bool,
    #[serde(default)]
    pub include_recommendations: bool,
}
```

## Proposed V2 state model

V2 should move beyond the current 3-dimension hidden state.

## Generic marketing state dimensions

These should be reusable across domains.

### Core state `z`

```text
z0 = hook_strength
z1 = credibility
z2 = clarity
z3 = conversion_intent
z4 = shareability
z5 = belonging
z6 = recommendation_confidence
```

Not every scenario has to use every dimension equally, but these dimensions map
to real marketing jobs better than:

- attention
- resonance
- share propensity

### Memory `m`

```text
m0 = fatigue
m1 = skepticism_residue
m2 = trust_accumulation
m3 = familiarity
```

### Uncertainty `u`

```text
u0 = audience_uncertainty
u1 = channel_uncertainty
u2 = conversion_uncertainty
u3 = general_model_uncertainty
```

## Channel-specific health indices

V2 should expose different headline metrics depending on scenario type.

### Universal metric families

```rust
pub enum MarketingMetric {
    ConsumerReceptivity,
    Credibility,
    ConversionIntent,
    ShareLikelihood,
    CommunityBelonging,
    RecommendationLift,
    PartnerProgression,
    RetentionFit,
    ObjectionPressure,
}
```

### Scenario-specific projections

#### Landing page

- clarity
- trust
- signup intent
- objection load

#### Short-form video

- hook strength
- save/share likelihood
- private-forward likelihood
- curiosity lift

#### In-store enablement

- budtender confidence
- story retention
- menu fit
- recommendation lift

#### Community event

- attendance intent
- repeat attendance
- belonging
- referral likelihood

#### Private relationship

- trust depth
- partner progression
- follow-up likelihood

## Proposed V2 result model

## Top-level result

```rust
pub struct MarketingSimulationResultV2 {
    pub simulation_id: String,
    pub approach_results: Vec<ApproachSimulationResultV2>,
    pub cross_approach_insights: Vec<String>,
    pub engine: EngineMetadata,
}
```

## Per-approach result

```rust
pub struct ApproachSimulationResultV2 {
    pub approach_id: String,
    pub primary_scores: PrimaryScorecard,
    pub metric_scores: Vec<MetricScore>,
    pub persona_breakdown: Vec<PersonaApproachResult>,
    pub top_reactions: Vec<String>,
    pub concerns: Vec<String>,
    pub failure_modes: Vec<String>,
    pub emergent_behaviors: Vec<String>,
    pub composure_archetype: String,
    pub run_summary: RunSummary,
    pub mean_trajectory: Vec<f64>,
    pub metric_trajectories: Option<Vec<MetricTrajectory>>,
    pub recommendations: Vec<String>,
}
```

## Primary scorecard

```rust
pub struct PrimaryScorecard {
    pub consumer_receptivity: u32,
    pub conversion_intent: u32,
    pub share_likelihood: u32,
    pub credibility: u32,
}
```

This replaces the current over-reliance on:

- engagement
- viral potential

Those can still exist as compatibility or derived fields.

## Metric score

```rust
pub struct MetricScore {
    pub metric: MarketingMetric,
    pub score: u32,
    pub confidence: u32,
}
```

## Persona breakdown

```rust
pub struct PersonaApproachResult {
    pub persona_id: String,
    pub weighted_score: u32,
    pub primary_scores: PrimaryScorecard,
    pub dominant_objections: Vec<String>,
    pub strongest_reactions: Vec<String>,
    pub fit_summary: String,
}
```

This is one of the highest-priority V2 outputs.

## Failure modes

Example failure mode strings:

- "Strong awareness, weak conversion because proof is insufficient"
- "Broad target weakens fit with highest-value segment"
- "Platform mismatch likely suppresses trust"
- "Belonging potential is high but repeatability is fragile"

## Recommendations

Example recommendation strings:

- "Test this against a more proof-heavy variant"
- "Keep target fixed and compare only hooks next"
- "Use this for private relationship building, not broad awareness"

## Backward compatibility

V2 should not require deleting v1 immediately.

## Recommended approach

### Step 1

Keep current structs and add:

- `MarketingSimulationRequestV2`
- `MarketingSimulationResultV2`

in parallel.

### Step 2

Expose:

- `simulate_marketing_v2(&MarketingSimulationRequestV2)`

alongside existing:

- `simulate_marketing(&MarketingSimulationRequest)`

### Step 3

Add CLI support:

```text
composure simulate-marketing-v2 <request.json>
```

### Step 4

Once downstream repos adopt V2, decide whether V1 stays as:

- compatibility mode
- simplified API
- or deprecated layer

## Suggested implementation order

## Milestone 1: V2 artifact and schema only

Implement:

- new request structs
- new result structs
- compatibility docs

No need to fully rewrite simulation logic yet.

## Milestone 2: richer derived metrics on top of current engine

Before changing state dimensions, derive:

- credibility
- conversion intent
- belonging
- recommendation lift

from the current trajectory plus richer profile signals.

This gives immediate value with limited risk.

## Milestone 3: persona-level execution

Run each approach per persona and then aggregate.

This is likely the single most valuable change in the whole V2 spec.

## Milestone 4: new state dimensions

Replace or generalize the internal 3-dimension state with the V2 marketing
state model.

## Milestone 5: scenario types

Add scenario-type-specific dynamics and scoring.

## Milestone 6: counterfactual and sequence support

Use existing composure foundations to compare:

- one approach vs another
- one sequence vs another

## What should stay out of V2

V2 should not yet include:

- LLM-driven persona generation inside the adapter
- project-specific business logic for AC or Mirrorlife
- full social graph simulation
- post-run freeform conversational agent layers

Those are better handled later or outside the core adapter.

## Success criteria

V2 is successful when:

1. AC can see which ideas work for:
   - wellness buyers
   - firefighters
   - budtenders
   - buyers
   - Vault prospects
2. Mirrorlife can see which ideas work for:
   - GLP-1 users
   - peptide stackers
   - privacy-sensitive users
   - premium users
3. The system outputs more than one or two generic scores
4. Results are explainable and tied to actual persona differences
5. The schema is reusable across both products without hacks

## Immediate next engineering task

After this doc, the next implementation step should be:

**add the V2 Rust types and a compatibility CLI path**

That gives us a concrete scaffold to build against without forcing a full engine
rewrite in one pass.
