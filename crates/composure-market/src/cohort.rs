//! Cohort aggregation — roll buyer outcomes up by archetype and signup timing bucket.

use crate::schemas::{BuyerArchetype, BuyerOutcome, BuyerState, CohortOutcome, MarketTotals};

/// Aggregation bucket key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CohortBucket {
    archetype: BuyerArchetype,
    signup_bin: usize, // signup week bucket (timestep / 7)
}

/// Build cohort outcomes from final buyer states and outcomes.
pub fn aggregate_cohorts(buyers: &[BuyerState], outcomes: &[BuyerOutcome]) -> Vec<CohortOutcome> {
    // Build a lookup: buyer_id -> outcome
    let outcomes_map: std::collections::HashMap<usize, &BuyerOutcome> =
        outcomes.iter().map(|o| (o.buyer_id, o)).collect();

    // Bucket buyers by archetype + signup week
    let mut buckets: std::collections::HashMap<CohortBucket, Vec<&BuyerState>> =
        std::collections::HashMap::new();

    for buyer in buyers {
        let signup_bin = if buyer.signup_t < 0 {
            // Never signed up — put in a special "no signup" bucket
            usize::MAX
        } else {
            (buyer.signup_t as usize) / 7
        };

        let bucket = CohortBucket {
            archetype: buyer.archetype,
            signup_bin,
        };

        buckets.entry(bucket).or_default().push(buyer);
    }

    let mut cohorts: Vec<CohortOutcome> = Vec::new();

    for (bucket, buyer_states) in buckets {
        let n = buyer_states.len();

        let signup_count = buyer_states.iter().filter(|b| b.signup_t >= 0).count();
        let activation_count = buyer_states.iter().filter(|b| b.activated_t >= 0).count();
        let churn_count = buyer_states.iter().filter(|b| b.churned_t >= 0).count();

        let signup_rate = if n > 0 {
            signup_count as f64 / n as f64
        } else {
            0.0
        };
        let activation_rate = if n > 0 {
            activation_count as f64 / n as f64
        } else {
            0.0
        };
        let churn_rate = if activation_count > 0 {
            churn_count as f64 / activation_count as f64
        } else {
            0.0
        };

        // LTV from outcomes
        let total_ltv: f64 = buyer_states
            .iter()
            .filter_map(|b| {
                outcomes_map
                    .get(&b.buyer_id)
                    .map(|o| o.lifetime_value_cents)
            })
            .sum();

        let avg_ltv = if n > 0 { total_ltv / n as f64 } else { 0.0 };

        let total_revenue = total_ltv;

        let referral_count: usize = buyer_states.iter().map(|b| b.referral_count).sum();

        let segment_key = if bucket.signup_bin == usize::MAX {
            format!("{:?} (no signup)", bucket.archetype)
        } else {
            format!("{:?} w{}", bucket.archetype, bucket.signup_bin)
        };

        cohorts.push(CohortOutcome {
            segment_key,
            archetype: bucket.archetype,
            buyer_count: n,
            signup_rate,
            activation_rate,
            churn_rate,
            avg_ltv_cents: avg_ltv,
            total_revenue_cents: total_revenue,
            referral_count,
        });
    }

    // Sort by archetype then signup bin for deterministic ordering
    cohorts.sort_by(|a, b| {
        let arch_order = archetype_order(&a.archetype).cmp(&archetype_order(&b.archetype));
        if arch_order != std::cmp::Ordering::Equal {
            arch_order
        } else {
            a.segment_key.cmp(&b.segment_key)
        }
    });

    cohorts
}

/// Compute market-wide totals from cohort data.
pub fn summarize_market(cohorts: &[CohortOutcome]) -> MarketTotals {
    let total_buyers: usize = cohorts.iter().map(|c| c.buyer_count).sum();
    let total_signups: usize = cohorts
        .iter()
        .map(|c| (c.signup_rate * c.buyer_count as f64).round() as usize)
        .sum();
    let total_activations: usize = cohorts
        .iter()
        .map(|c| (c.activation_rate * c.buyer_count as f64).round() as usize)
        .sum();
    let total_churns: usize = cohorts
        .iter()
        .map(|c| {
            let activations = (c.activation_rate * c.buyer_count as f64).round() as usize;
            (c.churn_rate * activations as f64).round() as usize
        })
        .sum();

    let total_referrals: usize = cohorts.iter().map(|c| c.referral_count).sum();
    let total_revenue_cents: f64 = cohorts.iter().map(|c| c.total_revenue_cents).sum();

    let market_ctr = if total_buyers > 0 {
        total_signups as f64 / total_buyers as f64
    } else {
        0.0
    };

    let market_cvr = if total_signups > 0 {
        total_activations as f64 / total_signups as f64
    } else {
        0.0
    };

    let market_ltv = if total_activations > 0 {
        total_revenue_cents / total_activations as f64
    } else {
        0.0
    };

    MarketTotals {
        total_buyers,
        total_signups,
        total_activations,
        total_churns,
        total_referrals,
        total_revenue_cents,
        market_ctr,
        market_cvr,
        market_ltv,
    }
}

fn archetype_order(a: &BuyerArchetype) -> usize {
    match a {
        BuyerArchetype::HighIntent => 0,
        BuyerArchetype::Browsers => 1,
        BuyerArchetype::DealSeekers => 2,
        BuyerArchetype::Loyalists => 3,
        BuyerArchetype::Dormant => 4,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schemas::BuyerState;

    fn make_state(
        id: usize,
        arch: BuyerArchetype,
        signup_t: i32,
        activated_t: i32,
        churned: bool,
    ) -> BuyerState {
        BuyerState {
            buyer_id: id,
            archetype: arch,
            aware: true,
            considering: true,
            signup_t,
            activated_t,
            churned_t: if churned { 10 } else { -1 },
            referral_count: 0,
            exposures: vec![],
        }
    }

    fn make_outcome(
        id: usize,
        arch: BuyerArchetype,
        signup: bool,
        activated: bool,
        ltv: f64,
    ) -> BuyerOutcome {
        BuyerOutcome {
            buyer_id: id,
            archetype: arch,
            reached_signup: signup,
            reached_activation: activated,
            churned: false,
            referral_count: 0,
            lifetime_value_cents: ltv,
            signup_t: if signup { 5 } else { -1 },
            activated_t: if activated { 7 } else { -1 },
        }
    }

    #[test]
    fn aggregate_cohorts_counts_match() {
        let states = vec![
            make_state(1, BuyerArchetype::HighIntent, 5, 7, false),
            make_state(2, BuyerArchetype::Browsers, -1, -1, false),
            make_state(3, BuyerArchetype::HighIntent, 3, 6, false),
        ];

        let outcomes = vec![
            make_outcome(1, BuyerArchetype::HighIntent, true, true, 8000.0),
            make_outcome(2, BuyerArchetype::Browsers, false, false, 0.0),
            make_outcome(3, BuyerArchetype::HighIntent, true, true, 9000.0),
        ];

        let cohorts = aggregate_cohorts(&states, &outcomes);
        let total: usize = cohorts.iter().map(|c| c.buyer_count).sum();
        assert_eq!(total, 3);
    }

    #[test]
    fn aggregate_cohorts_splits_by_archetype() {
        let states = vec![
            make_state(1, BuyerArchetype::HighIntent, 5, 7, false),
            make_state(2, BuyerArchetype::Loyalists, 3, 5, false),
        ];
        let outcomes = vec![
            make_outcome(1, BuyerArchetype::HighIntent, true, true, 8000.0),
            make_outcome(2, BuyerArchetype::Loyalists, true, true, 15000.0),
        ];
        let cohorts = aggregate_cohorts(&states, &outcomes);
        assert_eq!(cohorts.len(), 2);
    }

    #[test]
    fn aggregate_cohorts_buckets_never_signed_up() {
        let states = vec![
            make_state(1, BuyerArchetype::HighIntent, -1, -1, false),
            make_state(2, BuyerArchetype::HighIntent, 5, 7, false),
        ];
        let outcomes = vec![
            make_outcome(1, BuyerArchetype::HighIntent, false, false, 0.0),
            make_outcome(2, BuyerArchetype::HighIntent, true, true, 8000.0),
        ];
        let cohorts = aggregate_cohorts(&states, &outcomes);
        assert_eq!(cohorts.len(), 2); // no-signup bucket + signed-up bucket
    }

    #[test]
    fn summarize_market_zero_buyers() {
        let cohorts: Vec<CohortOutcome> = vec![];
        let totals = summarize_market(&cohorts);
        assert_eq!(totals.total_buyers, 0);
        assert_eq!(totals.market_ctr, 0.0);
        assert_eq!(totals.market_cvr, 0.0);
    }

    #[test]
    fn summarize_market_computes_averages() {
        let cohorts = vec![CohortOutcome {
            segment_key: "HighIntent w0".into(),
            archetype: BuyerArchetype::HighIntent,
            buyer_count: 10,
            signup_rate: 0.8,
            activation_rate: 0.6,
            churn_rate: 0.2,
            avg_ltv_cents: 8000.0,
            total_revenue_cents: 80_000.0,
            referral_count: 0,
        }];
        let totals = summarize_market(&cohorts);
        assert_eq!(totals.total_buyers, 10);
        assert_eq!(totals.total_signups, 8);
        assert_eq!(totals.total_activations, 6);
        assert!((totals.market_ctr - 0.8).abs() < 1e-6);
        assert!((totals.market_cvr - 0.75).abs() < 1e-6);
        assert!((totals.market_ltv - 80_000.0 / 6.0).abs() < 1e-2);
    }
}
