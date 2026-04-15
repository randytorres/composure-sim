//! Buyer state machine transitions.
//!
//! The buyer journey follows a deterministic state machine driven by
//! seeded random probability checks at each time step.
//!
//! State graph:
//! ```text
//! Unaware → Aware → Considering → SignedUp → Activated → Retained → (Churned | Referral)
//!                  ↓                                           ↓
//!              Dormant                                    Dormant (churn)
//! ```
//!
//! Transitions are monotonic — once a buyer churns they remain churned.

use crate::schemas::{BuyerArchetype, BuyerScores, BuyerState, CampaignVariant, Channel};
use rand::distributions::Distribution;
use rand::Rng;
use std::f64::consts::E;

/// How many time steps (on average) it takes an archetype to move from Aware → Considering.
fn consideration_lag_timesteps(archetype: BuyerArchetype) -> usize {
    match archetype {
        BuyerArchetype::HighIntent => 1,
        BuyerArchetype::Browsers => 4,
        BuyerArchetype::DealSeekers => 2,
        BuyerArchetype::Loyalists => 1,
        BuyerArchetype::Dormant => 3,
    }
}

/// Determine if a probabilistic transition fires.
///
/// Returns true with probability `p`. The check is deterministic given
/// the seeded RNG state — same RNG sequence always produces same outcome.
fn transition_fires<R: Rng>(p: f64, rng: &mut R) -> bool {
    rng.gen::<f64>() < p
}

/// Pick a channel based on variant's channel weights.
pub fn sample_channel<R: Rng>(variant: &CampaignVariant, rng: &mut R) -> Channel {
    let sampler = variant.channel_weights.sampler(rng);
    let idx = sampler.sample(rng);
    match idx {
        0 => Channel::Organic,
        1 => Channel::PaidSearch,
        2 => Channel::PaidSocial,
        3 => Channel::Email,
        4 => Channel::Referral,
        _ => Channel::Organic,
    }
}

/// Apply the unaware → aware transition for a single buyer at time step `t`.
///
/// Fires when the buyer's archetype base CTR × creative multiplier × spend
/// probability exceeds the RNG threshold.
pub fn step_unaware_to_aware<R: Rng>(
    buyer: &mut BuyerState,
    t: usize,
    variant: &CampaignVariant,
    rng: &mut R,
) {
    if buyer.aware {
        return;
    }

    let base_prob = variant.awareness_rate;
    let creative_mult = variant.creative_multipliers.get(buyer.archetype);
    // Probability increases slightly with time (campaign ramping)
    let time_factor = 1.0 + (t as f64 / variant.awareness_rate.max(1.0)) * 0.01;
    let p = (base_prob * creative_mult * time_factor).min(1.0);

    if transition_fires(p, rng) {
        buyer.aware = true;
        let channel = sample_channel(variant, rng);
        buyer.exposures.push(crate::schemas::ExposureRecord {
            timestep: t,
            channel,
            impressions: variant.impressions_per_exposure,
            spend_at_t: variant.spend_budget / variant.awareness_rate.max(1.0),
        });
    }
}

/// Apply the aware → considering transition.
///
/// Once aware, the buyer enters consideration with probability that grows
/// with time spent in the aware state.
pub fn step_aware_to_considering<R: Rng>(buyer: &mut BuyerState, _t: usize, rng: &mut R) {
    if !buyer.aware || buyer.considering {
        return;
    }

    let lag = consideration_lag_timesteps(buyer.archetype);
    // After `lag` timesteps, consideration becomes likely
    let exposure_count = buyer.exposures.len();
    let p = (exposure_count as f64 / lag as f64).min(1.0) * 0.8;

    if transition_fires(p, rng) {
        buyer.considering = true;
    }
}

/// Apply the considering → signup transition.
pub fn step_considering_to_signup<R: Rng>(buyer: &mut BuyerState, t: usize, rng: &mut R) {
    if buyer.signup_t >= 0 || !buyer.considering {
        return;
    }

    let base_rate = buyer.archetype.base_signup_rate();
    // Consideration bonus: buyers who have been considering longer are more likely
    let exposure_bonus = (buyer.exposures.len() as f64 * 0.02).min(0.2);
    let p = (base_rate + exposure_bonus).min(1.0);

    if transition_fires(p, rng) {
        buyer.signup_t = t as i32;
    }
}

/// Apply the signup → activated transition.
pub fn step_signup_to_activated<R: Rng>(buyer: &mut BuyerState, t: usize, rng: &mut R) {
    if buyer.activated_t >= 0 || buyer.signup_t < 0 {
        return;
    }

    // Activation lag: time since signup
    let days_since_signup = (t as i32 - buyer.signup_t) as usize;
    let base_rate = buyer.archetype.base_activation_rate();
    // Activation probability grows with time since signup
    let p = (base_rate * (days_since_signup as f64 + 1.0) / 7.0).min(1.0);

    if transition_fires(p, rng) {
        buyer.activated_t = t as i32;
    }
}

/// Apply the activated → retained (and churn) transitions per time step.
pub fn step_activated<R: Rng>(buyer: &mut BuyerState, t: usize, rng: &mut R) {
    if buyer.activated_t < 0 || buyer.churned_t >= 0 {
        return;
    }

    let days_since_activation = (t as i32 - buyer.activated_t) as usize;
    let base_churn = buyer.archetype.base_weekly_churn();
    // Churn probability increases slightly over time (aging factor)
    let aging_factor = 1.0 + (days_since_activation as f64 / 30.0) * 0.1;
    let churn_p = (base_churn * aging_factor).min(0.5); // cap at 50% per timestep

    if transition_fires(churn_p, rng) {
        buyer.churned_t = t as i32;
    }
}

/// Apply the retained → referral transition.
pub fn step_referral<R: Rng>(buyer: &mut BuyerState, t: usize, rng: &mut R) {
    // Only active retained buyers can refer
    if buyer.churned_t >= 0 || buyer.activated_t < 0 {
        return;
    }

    let base_rate = buyer.archetype.base_share_rate();
    // Referrals get harder over time (enthusiasm fades)
    let days_since_activation = (t as i32 - buyer.activated_t) as usize;
    let decay = E.powf(-0.05 * (days_since_activation as f64));
    let p = base_rate * decay;

    if transition_fires(p, rng) {
        buyer.referral_count += 1;
        // Referral bonus: referred buyers get a small awareness boost
        // (handled externally by the engine when generating new exposures)
    }
}

/// Score all transition probabilities for a buyer at time `t`.
///
/// These scores are used both for analytics/reporting and for actual
/// transition decisions (via the same seeded RNG).
pub fn score_buyer<R: Rng>(
    buyer: &BuyerState,
    t: usize,
    variant: &CampaignVariant,
    _rng: &mut R,
) -> BuyerScores {
    use crate::schemas::BuyerScores;

    let click_p = if !buyer.aware {
        let base = variant.awareness_rate;
        let creative = variant.creative_multipliers.get(buyer.archetype);
        (base * creative * (1.0 + t as f64 * 0.001)).min(1.0)
    } else {
        0.0
    };

    let signup_p = if buyer.considering && buyer.signup_t < 0 {
        let base = buyer.archetype.base_signup_rate();
        let exposure_bonus = (buyer.exposures.len() as f64 * 0.02).min(0.2);
        (base + exposure_bonus).min(1.0)
    } else {
        0.0
    };

    let activation_p = if buyer.signup_t >= 0 && buyer.activated_t < 0 {
        let days = (t as i32 - buyer.signup_t).max(0) as usize;
        let base = buyer.archetype.base_activation_rate();
        (base * (days + 1) as f64 / 7.0).min(1.0)
    } else {
        0.0
    };

    let retention_p = if buyer.activated_t >= 0 && buyer.churned_t < 0 {
        let days = (t as i32 - buyer.activated_t).max(0) as usize;
        let base = buyer.archetype.base_weekly_retention();
        base * E.powf(-0.01 * (days as f64))
    } else if buyer.activated_t < 0 {
        1.0 // not yet activated, assume retained
    } else {
        0.0
    };

    let churn_p = if buyer.activated_t >= 0 && buyer.churned_t < 0 {
        let days = (t as i32 - buyer.activated_t).max(0) as usize;
        let base = buyer.archetype.base_weekly_churn();
        (base * (1.0 + days as f64 / 30.0) * 0.1).min(0.5)
    } else {
        0.0
    };

    let share_p = if buyer.activated_t >= 0 && buyer.churned_t < 0 {
        let days = (t as i32 - buyer.activated_t).max(0) as usize;
        let base = buyer.archetype.base_share_rate();
        base * E.powf(-0.05 * (days as f64))
    } else {
        0.0
    };

    BuyerScores {
        click_probability: click_p,
        signup_probability: signup_p,
        activation_probability: activation_p,
        retention_probability: retention_p,
        churn_probability: churn_p,
        share_probability: share_p,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    fn test_variant() -> CampaignVariant {
        CampaignVariant {
            variant_id: "test".into(),
            spend_budget: 10_000.0,
            ..Default::default()
        }
    }

    #[test]
    fn transition_fires_never_at_zero() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        for _ in 0..1000 {
            assert!(!transition_fires(0.0, &mut rng));
        }
    }

    #[test]
    fn transition_fires_always_at_one() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        for _ in 0..1000 {
            assert!(transition_fires(1.0, &mut rng));
        }
    }

    #[test]
    fn score_buyer_unaware_returns_zero_signup() {
        let buyer = BuyerState::new(1, BuyerArchetype::HighIntent);
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        let scores = score_buyer(&buyer, 0, &test_variant(), &mut rng);
        assert_eq!(scores.signup_probability, 0.0);
        assert_eq!(scores.activation_probability, 0.0);
        assert_eq!(scores.share_probability, 0.0);
    }

    #[test]
    fn score_buyer_considering_returns_signup_probability() {
        let mut buyer = BuyerState::new(1, BuyerArchetype::HighIntent);
        buyer.aware = true;
        buyer.considering = true;
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        let scores = score_buyer(&buyer, 0, &test_variant(), &mut rng);
        assert!(scores.signup_probability > 0.0);
    }

    #[test]
    fn score_buyer_activated_returns_retention_probability() {
        let mut buyer = BuyerState::new(1, BuyerArchetype::Loyalists);
        buyer.aware = true;
        buyer.considering = true;
        buyer.signup_t = 0;
        buyer.activated_t = 1;
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        let scores = score_buyer(&buyer, 5, &test_variant(), &mut rng);
        assert!(scores.retention_probability > 0.0);
        assert!(scores.share_probability > 0.0);
    }
}
