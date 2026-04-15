//! # composure-population
//!
//! Synthetic buyer population system for the composure simulation engine.
//!
//! Provides:
//! - [`SegmentBlueprint`]: Reusable schema for a buyer segment with priors, traits,
//!   channel preferences, objections, budget, trust, and product-friction tolerances.
//! - [`TraitDistribution`]: Parametric trait distributions (Uniform, Normal, Beta,
//!   TruncatedNormal, Categorical).
//! - [`CorrelatedTraitSampler`]: Multivariate correlated trait sampling via Cholesky
//!   decomposition for realistic within-segment heterogeneity.
//! - [`PopulationGenerator`]: Expand a set of blueprints into 10k–100k buyers with
//!   stable IDs and seeded determinism.
//! - [`Buyer`]: Individual buyer with sampled traits, memory state, and social graph edges.
//! - [`SocialGraph`]: Graph generator supporting friend clusters, creator-follow, and
//!   community adjacency by channel.
//! - [`InfluencePropagator`]: Spread exposure, proof, skepticism, and referrals across
//!   the social graph over time.
//! - [`StageTransition`]: Buyer segment migration rules (curious observer → active
//!   tracker → churn-risk skeptic).
//! - [`SampledTranscript`]: Human-readable reaction summaries for stratified buyer subsets.

pub mod blueprint;
pub mod buyer;
pub mod correlated;
pub mod influence;
pub mod population;
pub mod social_graph;
pub mod stage;
pub mod traits;
pub mod transcript;

pub use blueprint::{
    ChannelPreference, Objection, ObjectionType, Prior, ProductFriction, SegmentBlueprint,
    SegmentStage, TraitDistributionConfig,
};
pub use buyer::Buyer;
pub use correlated::CorrelatedTraitSampler;
pub use influence::InfluencePropagator;
pub use population::{PopulationConfig, PopulationGenerator, SyntheticPopulationSnapshot};
pub use social_graph::{Edge, EdgeKind, SocialGraph, SocialGraphConfig, SocialGraphGenerator};
pub use stage::{StageTransitionEngine, TransitionCondition, TransitionRule};
pub use traits::{TraitDistribution, TraitSampler, TraitValue};
pub use transcript::{build_transcripts, BuyerTranscript, TranscriptConfig, TranscriptLine};
