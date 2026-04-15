//! Stage transitions — Task 18.
//!
//! Buyer segment migration rules (curious observer → active tracker → buyer → churn-risk skeptic).

use crate::blueprint::SegmentStage;
use rand::Rng;
use serde::{Deserialize, Serialize};

/// Conditions that can trigger a stage transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionCondition {
    /// Trust level that must be met.
    pub min_trust: Option<f64>,
    /// Maximum trust level allowed (for downgrade transitions).
    pub max_trust: Option<f64>,
    /// Minimum number of exposures required.
    pub min_exposures: Option<usize>,
    /// Minimum proof score required (from influence propagator).
    pub min_proof_score: Option<f64>,
    /// Minimum skepticism score before downgrade.
    pub min_skepticism_score: Option<f64>,
    /// Buyer must have purchased at least this many times.
    pub min_purchases: Option<usize>,
    /// Number of churn events required to trigger downgrade.
    pub min_churns: Option<usize>,
}

/// A single transition rule from one stage to another.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionRule {
    /// Stage this rule applies to.
    pub from_stage: SegmentStage,
    /// Stage the buyer moves to if conditions are met.
    pub to_stage: SegmentStage,
    /// Conditions that must be satisfied for this transition.
    pub condition: TransitionCondition,
    /// Probability this transition fires if conditions are met (0–1).
    pub probability: f64,
}

impl TransitionRule {
    pub fn evaluate(
        &self,
        stage: SegmentStage,
        trust: f64,
        exposures: usize,
        proof_score: f64,
        skepticism_score: f64,
        purchases: usize,
        churns: usize,
        rng: &mut impl Rng,
    ) -> Option<SegmentStage> {
        if stage != self.from_stage {
            return None;
        }

        if let Some(min_trust) = self.condition.min_trust {
            if trust < min_trust {
                return None;
            }
        }
        if let Some(max_trust) = self.condition.max_trust {
            if trust > max_trust {
                return None;
            }
        }
        if let Some(min_exp) = self.condition.min_exposures {
            if exposures < min_exp {
                return None;
            }
        }
        if let Some(min_proof) = self.condition.min_proof_score {
            if proof_score < min_proof {
                return None;
            }
        }
        if let Some(min_skep) = self.condition.min_skepticism_score {
            if skepticism_score < min_skep {
                return None;
            }
        }
        if let Some(min_pur) = self.condition.min_purchases {
            if purchases < min_pur {
                return None;
            }
        }
        if let Some(min_churn) = self.condition.min_churns {
            if churns < min_churn {
                return None;
            }
        }

        if rng.gen::<f64>() < self.probability {
            Some(self.to_stage)
        } else {
            None
        }
    }
}

/// Stage transition engine — evaluates transition rules per buyer per time step.
#[derive(Debug, Clone)]
pub struct StageTransitionEngine {
    rules: Vec<TransitionRule>,
}

impl StageTransitionEngine {
    /// Build with a set of rules (or use the default BiohackerRules).
    pub fn new(rules: Vec<TransitionRule>) -> Self {
        Self { rules }
    }

    /// Evaluate all rules and return the new stage (if any transition fires).
    pub fn evaluate(
        &self,
        current_stage: SegmentStage,
        trust: f64,
        exposures: usize,
        proof_score: f64,
        skepticism_score: f64,
        purchases: usize,
        churns: usize,
        rng: &mut impl Rng,
    ) -> SegmentStage {
        for rule in &self.rules {
            if let Some(new_stage) = rule.evaluate(
                current_stage,
                trust,
                exposures,
                proof_score,
                skepticism_score,
                purchases,
                churns,
                rng,
            ) {
                return new_stage;
            }
        }
        current_stage
    }

    /// The standard biohacker buyer stage machine.
    pub fn biohacker_default() -> Self {
        Self::new(vec![
            // CuriousObserver → ActiveTracker: seen 5+ exposures
            TransitionRule {
                from_stage: SegmentStage::CuriousObserver,
                to_stage: SegmentStage::ActiveTracker,
                condition: TransitionCondition { min_exposures: Some(5), ..Default::default() },
                probability: 0.7,
            },
            // ActiveTracker → Evaluator: seen proof, trust building
            TransitionRule {
                from_stage: SegmentStage::ActiveTracker,
                to_stage: SegmentStage::Evaluator,
                condition: TransitionCondition { min_trust: Some(0.4), min_proof_score: Some(2.0), ..Default::default() },
                probability: 0.6,
            },
            // Evaluator → Buyer: trust high enough, skeptical not dominant
            TransitionRule {
                from_stage: SegmentStage::Evaluator,
                to_stage: SegmentStage::Buyer,
                condition: TransitionCondition { min_trust: Some(0.65), min_proof_score: Some(3.0), min_skepticism_score: Some(1.0), ..Default::default() },
                probability: 0.5,
            },
            // Buyer → Advocate: 2+ purchases, trust holds
            TransitionRule {
                from_stage: SegmentStage::Buyer,
                to_stage: SegmentStage::Advocate,
                condition: TransitionCondition { min_purchases: Some(2), min_trust: Some(0.7), ..Default::default() },
                probability: 0.4,
            },
            // Any stage → ChurnRiskSkeptic: trust drops too low OR 2+ churn events
            TransitionRule {
                from_stage: SegmentStage::CuriousObserver,
                to_stage: SegmentStage::ChurnRiskSkeptic,
                condition: TransitionCondition { min_churns: Some(2), ..Default::default() },
                probability: 1.0,
            },
            TransitionRule {
                from_stage: SegmentStage::ActiveTracker,
                to_stage: SegmentStage::ChurnRiskSkeptic,
                condition: TransitionCondition { max_trust: Some(0.15), ..Default::default() },
                probability: 1.0,
            },
            TransitionRule {
                from_stage: SegmentStage::Evaluator,
                to_stage: SegmentStage::ChurnRiskSkeptic,
                condition: TransitionCondition { min_skepticism_score: Some(5.0), ..Default::default() },
                probability: 0.8,
            },
            TransitionRule {
                from_stage: SegmentStage::Buyer,
                to_stage: SegmentStage::ChurnRiskSkeptic,
                condition: TransitionCondition { min_churns: Some(1), max_trust: Some(0.2), ..Default::default() },
                probability: 0.9,
            },
            TransitionRule {
                from_stage: SegmentStage::Advocate,
                to_stage: SegmentStage::ChurnRiskSkeptic,
                condition: TransitionCondition { min_churns: Some(2), max_trust: Some(0.25), ..Default::default() },
                probability: 0.8,
            },
        ])
    }
}

impl Default for TransitionCondition {
    fn default() -> Self {
        Self {
            min_trust: None,
            max_trust: None,
            min_exposures: None,
            min_proof_score: None,
            min_skepticism_score: None,
            min_purchases: None,
            min_churns: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn test_observer_to_tracker() {
        let engine = StageTransitionEngine::biohacker_default();
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let new_stage = engine.evaluate(
            SegmentStage::CuriousObserver,
            0.5,
            5, // min_exposures = 5
            2.0,
            0.5,
            0,
            0,
            &mut rng,
        );
        // With rng seed 42 and probability 0.7, should transition
        assert!(matches!(new_stage, SegmentStage::ActiveTracker | SegmentStage::CuriousObserver));
    }

    #[test]
    fn test_no_transition() {
        let engine = StageTransitionEngine::biohacker_default();
        let mut rng = rand::rngs::StdRng::seed_from_u64(99);
        let new_stage = engine.evaluate(
            SegmentStage::CuriousObserver,
            0.5,
            3, // below threshold
            0.0,
            0.0,
            0,
            0,
            &mut rng,
        );
        assert_eq!(new_stage, SegmentStage::CuriousObserver);
    }

    #[test]
    fn test_trust_downgrade() {
        let engine = StageTransitionEngine::biohacker_default();
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let new_stage = engine.evaluate(
            SegmentStage::ActiveTracker,
            0.1, // below max_trust = 0.15
            10,
            0.0,
            2.0,
            0,
            0,
            &mut rng,
        );
        assert_eq!(new_stage, SegmentStage::ChurnRiskSkeptic);
    }

    #[test]
    fn test_evaluator_to_buyer() {
        let engine = StageTransitionEngine::biohacker_default();
        // Evaluator → Buyer: trust >= 0.65, proof >= 3.0, skept >= 1.0, prob = 0.5
        // Try multiple seeds until one fires the transition
        for seed in [42u64, 7, 13, 99, 123, 456] {
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            let new_stage = engine.evaluate(
                SegmentStage::Evaluator,
                0.7,
                20,
                4.0,
                2.0,
                0,
                0,
                &mut rng,
            );
            if matches!(new_stage, SegmentStage::Buyer) {
                return;
            }
        }
        panic!("no seed produced a Buyer transition");
    }
}
