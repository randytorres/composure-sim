//! # composure-market
//!
//! Buyer-level market simulation kernel for `composure-sim`.
//!
//! Provides deterministic, seeded simulation of buyer populations with
//! persistent state across time steps, per-buyer scoring, and cohort-level
//! aggregation.
//!
//! ## Key types
//!
//! - [`MarketSimulationConfig`] — input config with population + campaign variants
//! - [`MarketSimEngine`] — seeded runner that produces [`MarketSimulationResult`]
//! - [`BuyerOutcome`] — per-buyer outcome at simulation end
//! - [`CohortOutcome`] — aggregated by archetype + signup timing bucket
//! - [`MarketTotals`] — market-wide KPIs
//!
//! ## Example
//!
//! ```rust
//! use composure_market::{MarketSimEngine, MarketSimulationConfig};
//!
//! let config = MarketSimulationConfig::default();
//! let mut engine = MarketSimEngine::new(config);
//! let result = engine.run();
//! println!("Total signups: {}", result.market_totals.total_signups);
//! ```

pub mod cohort;
pub mod engine;
pub mod outputs;
pub mod schemas;
pub mod transitions;

pub use cohort::{aggregate_cohorts, summarize_market};
pub use engine::MarketSimEngine;
pub use outputs::build_result;
pub use schemas::{
    config_digest, BuyerArchetype, BuyerOutcome, BuyerScores, BuyerState, CampaignVariant, Channel,
    ChannelWeights, CohortOutcome, ConversionEvent, ConversionEventType, CreativeMultipliers,
    MarketSimulationConfig, MarketSimulationResult, MarketTotals, SyntheticPopulationConfig,
    Validate, ValidationError, ARCHETYPE_VARIANTS,
};
