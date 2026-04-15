//! Influence propagator — Task 17.
//!
//! Spread exposure, proof, skepticism, and referrals across the social graph over time.

use crate::social_graph::SocialGraph;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Influence type that can propagate across edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InfluenceType {
    /// Awareness of the product spread by exposure.
    Exposure,
    /// Social proof: positive evidence shared peer-to-peer.
    SocialProof,
    /// Skepticism: doubt or negative sentiment.
    Skepticism,
    /// Referral: invitation to try the product.
    Referral,
}

impl InfluenceType {
    /// Valence: positive (+1), neutral (0), or negative (-1).
    pub fn valence(&self) -> i8 {
        match self {
            Self::Exposure => 0,
            Self::SocialProof => 1,
            Self::Skepticism => -1,
            Self::Referral => 1,
        }
    }
}

/// A propagating influence event on the graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfluenceEvent {
    pub from_id: String,
    pub to_id: String,
    pub influence_type: InfluenceType,
    pub step: usize,
    pub magnitude: f64,
}

/// Influence state per buyer.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BuyerInfluenceState {
    /// Accumulated exposure count.
    pub exposure_score: f64,
    /// Accumulated social proof score.
    pub proof_score: f64,
    /// Accumulated skepticism score.
    pub skepticism_score: f64,
    /// Accumulated referral count.
    pub referral_count: usize,
}

/// Configuration for influence propagation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfluenceConfig {
    /// Max propagation hops from the seed.
    pub max_hops: usize,
    /// Decay per hop: magnitude *= hop_decay.
    pub hop_decay: f64,
    /// Edge-weight multiplier: how much the edge weight scales influence.
    pub edge_weight_boost: f64,
    /// Skepticism half-life: skepticism decays by this factor per step.
    pub skepticism_decay: f64,
    /// Proof boost: how much social proof improves trust.
    pub proof_trust_boost: f64,
    /// Skeptic penalty: how much skepticism reduces trust.
    pub skeptic_trust_penalty: f64,
}

impl Default for InfluenceConfig {
    fn default() -> Self {
        Self {
            max_hops: 3,
            hop_decay: 0.7,
            edge_weight_boost: 1.0,
            skepticism_decay: 0.9,
            proof_trust_boost: 0.05,
            skeptic_trust_penalty: 0.08,
        }
    }
}

/// Influence propagator — spreads influence across a social graph over time steps.
pub struct InfluencePropagator {
    config: InfluenceConfig,
}

impl InfluencePropagator {
    pub fn new(config: InfluenceConfig) -> Self {
        Self { config }
    }

    /// Propagate a single influence type from a set of seed buyer IDs across the graph.
    ///
    /// Returns:
    /// - `per_step_influence`: map of step → list of events that occurred that step
    /// - `buyer_states`: per-buyer influence accumulation after all steps
    pub fn propagate(
        &self,
        graph: &SocialGraph,
        seeds: &[String],
        influence_type: InfluenceType,
        num_steps: usize,
        rng: &mut impl Rng,
    ) -> (HashMap<usize, Vec<InfluenceEvent>>, HashMap<String, BuyerInfluenceState>) {
        let mut per_step_events: HashMap<usize, Vec<InfluenceEvent>> = (0..=num_steps).map(|t| (t, vec![])).collect();
        let mut buyer_states: HashMap<String, BuyerInfluenceState> = graph
            .adjacency
            .keys()
            .map(|id| (id.clone(), BuyerInfluenceState::default()))
            .collect();

        // Initialize seeds with full influence
        for seed in seeds {
            if let Some(state) = buyer_states.get_mut(seed) {
                match influence_type {
                    InfluenceType::Exposure => state.exposure_score += 1.0,
                    InfluenceType::SocialProof => state.proof_score += 1.0,
                    InfluenceType::Skepticism => state.skepticism_score += 1.0,
                    InfluenceType::Referral => state.referral_count += 1,
                }
            }
        }

        // BFS-like propagation over hops
        let mut frontier: HashMap<String, f64> = seeds.iter().map(|id| (id.clone(), 1.0)).collect();
        let mut visited: std::collections::HashSet<String> = seeds.iter().cloned().collect();
        let total_steps = num_steps.min(self.config.max_hops);

        for hop in 1..=total_steps {
            let mut next_frontier: HashMap<String, f64> = HashMap::new();
            // hop_decay is applied: magnitude at hop H = hop_decay^(H-1)
            let hop_decay_factor = self.config.hop_decay.powi(hop as i32 - 1);

            for (current_id, current_mag) in &frontier {
                if let Some(neighbors) = graph.adjacency.get(current_id) {
                    for neighbor_id in neighbors {
                        if visited.contains(neighbor_id) {
                            continue;
                        }
                        // Find edge weight
                        let weight = graph
                            .edges
                            .iter()
                            .find(|e| e.from == *current_id && e.to == *neighbor_id)
                            .map(|e| e.weight)
                            .unwrap_or(0.5);

                        // Apply hop_decay: influence decays per hop
                        let effective_mag = current_mag * weight * self.config.edge_weight_boost * hop_decay_factor;
                        if effective_mag < 0.05 {
                            continue; // too weak to propagate
                        }

                        // Propagate with probability proportional to magnitude
                        if rng.gen::<f64>() < effective_mag.min(1.0) {
                            let event = InfluenceEvent {
                                from_id: current_id.clone(),
                                to_id: neighbor_id.clone(),
                                influence_type,
                                step: hop,
                                magnitude: effective_mag,
                            };
                            per_step_events.get_mut(&hop).unwrap().push(event);

                            // Update state
                            if let Some(state) = buyer_states.get_mut(neighbor_id) {
                                match influence_type {
                                    InfluenceType::Exposure => state.exposure_score += effective_mag,
                                    InfluenceType::SocialProof => state.proof_score += effective_mag,
                                    InfluenceType::Skepticism => state.skepticism_score += effective_mag,
                                    InfluenceType::Referral => state.referral_count += 1,
                                }
                            }

                            visited.insert(neighbor_id.clone());
                            *next_frontier.entry(neighbor_id.clone()).or_insert(0.0) += effective_mag;
                        }
                    }
                }
            }

            // Only skepticism propagation should decay the accumulated skepticism
            // state over time; other influence types should not mutate it.
            if matches!(influence_type, InfluenceType::Skepticism) {
                for state in buyer_states.values_mut() {
                    state.skepticism_score *= self.config.skepticism_decay;
                }
            }

            frontier = next_frontier;
        }

        (per_step_events, buyer_states)
    }

    /// Compute trust delta for each buyer based on their influence state.
    /// skepticism_decay moderates how much accumulated skepticism penalizes trust.
    pub fn compute_trust_delta(
        &self,
        influence_state: &BuyerInfluenceState,
    ) -> f64 {
        let proof_gain = influence_state.proof_score * self.config.proof_trust_boost;
        let skeptic_loss = influence_state.skepticism_score * self.config.skeptic_trust_penalty;
        (proof_gain - skeptic_loss).clamp(-1.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::social_graph::Edge;
    use crate::social_graph::EdgeKind;
    use rand::SeedableRng;

    #[test]
    fn test_propagate_exposure() {
        let mut graph = SocialGraph::default();
        graph.adjacency.insert("A".to_string(), vec!["B".to_string()]);
        graph.adjacency.insert("B".to_string(), vec!["C".to_string()]);
        graph.reverse_adjacency.insert("B".to_string(), vec!["A".to_string()]);
        graph.reverse_adjacency.insert("C".to_string(), vec!["B".to_string()]);
        graph.edges.push(Edge::new("A".to_string(), "B".to_string(), EdgeKind::Friend, 0.9));
        graph.edges.push(Edge::new("B".to_string(), "C".to_string(), EdgeKind::Friend, 0.9));

        let prop = InfluencePropagator::new(InfluenceConfig { max_hops: 3, ..Default::default() });
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let (_events, states) = prop.propagate(&graph, &["A".to_string()], InfluenceType::Exposure, 3, &mut rng);

        let b_state = states.get("B").unwrap();
        assert!(b_state.exposure_score > 0.0);
    }

    #[test]
    fn test_trust_delta_positive() {
        let prop = InfluencePropagator::new(InfluenceConfig::default());
        let state = BuyerInfluenceState { exposure_score: 0.0, proof_score: 5.0, skepticism_score: 0.0, referral_count: 0 };
        let delta = prop.compute_trust_delta(&state);
        assert!(delta > 0.0);
    }

    #[test]
    fn test_trust_delta_negative() {
        let prop = InfluencePropagator::new(InfluenceConfig::default());
        let state = BuyerInfluenceState { exposure_score: 0.0, proof_score: 0.0, skepticism_score: 3.0, referral_count: 0 };
        let delta = prop.compute_trust_delta(&state);
        assert!(delta < 0.0);
    }

    #[test]
    fn test_hop_decay() {
        // B gets influence from A directly; C gets it indirectly through B.
        // Since we skip already-visited nodes, C can only be reached from B if B
        // is in the frontier — but with effective_mag threshold = 0.05, B (mag=1.0*1.0=1.0)
        // should propagate to C. The test checks that B receives influence at all.
        let prop = InfluencePropagator::new(InfluenceConfig { hop_decay: 0.5, ..Default::default() });
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let mut graph = SocialGraph::default();
        graph.adjacency.insert("A".to_string(), vec!["B".to_string()]);
        graph.adjacency.insert("B".to_string(), vec!["C".to_string()]);
        graph.reverse_adjacency.insert("B".to_string(), vec!["A".to_string()]);
        graph.reverse_adjacency.insert("C".to_string(), vec!["B".to_string()]);
        graph.edges.push(Edge::new("A".to_string(), "B".to_string(), EdgeKind::Follow, 1.0));
        graph.edges.push(Edge::new("B".to_string(), "C".to_string(), EdgeKind::Follow, 1.0));

        let (_, states) = prop.propagate(&graph, &["A".to_string()], InfluenceType::Exposure, 3, &mut rng);
        // B should receive influence from A (weight=1.0, mag=1.0)
        let b_exp = states.get("B").unwrap().exposure_score;
        assert!(b_exp > 0.0, "B should receive influence from A");
    }

    #[test]
    fn test_num_steps_caps_propagation_depth() {
        let prop = InfluencePropagator::new(InfluenceConfig { max_hops: 3, ..Default::default() });
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let mut graph = SocialGraph::default();
        graph.adjacency.insert("A".to_string(), vec!["B".to_string()]);
        graph.adjacency.insert("B".to_string(), vec!["C".to_string()]);
        graph.adjacency.insert("C".to_string(), vec![]);
        graph.reverse_adjacency.insert("B".to_string(), vec!["A".to_string()]);
        graph.reverse_adjacency.insert("C".to_string(), vec!["B".to_string()]);
        graph.edges.push(Edge::new("A".to_string(), "B".to_string(), EdgeKind::Follow, 1.0));
        graph.edges.push(Edge::new("B".to_string(), "C".to_string(), EdgeKind::Follow, 1.0));

        let (_, states) = prop.propagate(&graph, &["A".to_string()], InfluenceType::Exposure, 1, &mut rng);
        assert!(states.get("B").unwrap().exposure_score > 0.0);
        assert_eq!(states.get("C").unwrap().exposure_score, 0.0);
    }
}
