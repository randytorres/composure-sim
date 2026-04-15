//! Sampled transcript layer — Task 19.
//!
//! Generates human-readable reaction summaries for a stratified subset of buyers.
//! NOT all 10k–100k buyers get transcripts — only the sampled set.

use rand::{prelude::*, Rng, SeedableRng};
use serde::{Deserialize, Serialize};

/// A human-readable reaction transcript for a single buyer at a single time step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptLine {
    pub buyer_id: String,
    pub step: usize,
    pub stage: String,
    pub trust: f64,
    pub reaction: String,
    pub channel: String,
    pub influence_type: Option<String>,
}

/// A full transcript for a sampled buyer.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BuyerTranscript {
    pub buyer_id: String,
    pub segment_id: String,
    pub lines: Vec<TranscriptLine>,
    /// Summary label derived from the overall arc.
    pub arc_summary: String,
}

impl BuyerTranscript {
    pub fn append(&mut self, line: TranscriptLine) {
        self.lines.push(line);
    }

    pub fn build_arc_summary(&self) -> String {
        if self.lines.is_empty() {
            return "No interactions recorded".to_string();
        }
        let first = &self.lines[0];
        let last = &self.lines[self.lines.len() - 1];

        let trust_delta = last.trust - first.trust;
        let stages: Vec<_> = self.lines.iter().map(|l| l.stage.as_str()).collect();
        let unique_stages: Vec<_> = stages.iter().collect::<std::collections::HashSet<_>>().iter().copied().collect();

        let progression = if unique_stages.len() > 1 {
            "evolving"
        } else {
            "stable"
        };

        let trust_direction = if trust_delta > 0.1 {
            "trust-building"
        } else if trust_delta < -0.1 {
            "trust-erosion"
        } else {
            "trust-holding"
        };

        format!("{} {} {} buyer", progression, trust_direction, last.stage)
    }
}

/// Configuration for transcript sampling.
#[derive(Debug, Clone)]
pub struct TranscriptConfig {
    /// Fraction of buyers to generate transcripts for [0, 1].
    pub sample_rate: f64,
    /// Seed for transcript sampling.
    pub seed: u64,
    /// Maximum lines per transcript (-1 = unlimited).
    pub max_lines: isize,
}

impl Default for TranscriptConfig {
    fn default() -> Self {
        Self { sample_rate: 0.01, seed: 42, max_lines: -1 }
    }
}

/// Template phrases for generating reactions.
fn sample_reaction(_buyer_id: &str, _step: usize, trust: f64, channel: &str, influence_type: Option<&str>, rng: &mut impl Rng) -> String {
    let trust_bucket = if trust > 0.7 {
        "high"
    } else if trust > 0.4 {
        "medium"
    } else if trust > 0.2 {
        "low"
    } else {
        "critical"
    };

    let reactions_high = [
        "Shared the product link with three friends",
        "Left a positive review citing visible results",
        "Asked a detailed question about the peptide protocol",
        "Mentioned they feel noticeably better energy levels",
        "Forwarded the community post to their network",
    ];
    let reactions_medium = [
        "Read the case study and is evaluating the tradeoffs",
        "Bookmarked the comparison page for later review",
        "Saw the peer testimonial and is reconsidering their skepticism",
        "Engaged with the Instagram reel twice this week",
        "Asked whether the wearable data correlates with outcomes",
    ];
    let reactions_low = [
        "Scrolled past the ad without clicking",
        "Expressed concern about peer-reviewed evidence gap",
        "Declined the referral link citing price",
        "Reported uncertainty about safety profile",
        "Lost to a competitor product in their mind",
    ];
    let reactions_critical = [
        "Activated churn objection (price + side effects narrative)",
        "Left a negative comment on the creator video",
        "Reported data privacy concern to their network",
        "Switched to a competing peptide provider",
        "Unsubscribe triggered by trust collapse",
    ];

    let reactions = match trust_bucket {
        "high" => &reactions_high,
        "medium" => &reactions_medium,
        "low" => &reactions_low,
        _ => &reactions_critical,
    };

    let base = reactions[rng.gen::<usize>() % reactions.len()];

    match influence_type {
        Some(it) => format!("[{} via {}] {}", channel, it, base),
        None => format!("[{}] {}", channel, base),
    }
}

/// Build sampled transcripts for a subset of buyers.
pub fn build_transcripts(
    buyer_ids: &[String],
    segment_ids: &[String],
    snapshot: &crate::population::SyntheticPopulationSnapshot,
    influence_states: &std::collections::HashMap<String, crate::influence::BuyerInfluenceState>,
    steps: usize,
    config: &TranscriptConfig,
) -> Vec<BuyerTranscript> {
    let mut rng = rand_chacha::ChaCha12Rng::seed_from_u64(config.seed);
    let sample_size = (buyer_ids.len() as f64 * config.sample_rate) as usize;
    let mut sampled: Vec<usize> = (0..buyer_ids.len()).collect();
    sampled.shuffle(&mut rng);
    let sampled: Vec<_> = sampled.into_iter().take(sample_size).collect();

    sampled
        .iter()
        .map(|&idx| {
            let id = &buyer_ids[idx];
            let segment = &segment_ids[idx];
            let buyer = snapshot.buyers.iter().find(|b| &b.id.0 == id).cloned().unwrap_or_default();

            let mut transcript = BuyerTranscript {
                buyer_id: id.clone(),
                segment_id: segment.clone(),
                lines: vec![],
                arc_summary: String::new(),
            };

            let max_lines = if config.max_lines < 0 { steps } else { config.max_lines as usize };
            for step in 0..steps.min(max_lines) {
                let default_state = crate::influence::BuyerInfluenceState::default();
                let inf_state = influence_states.get(id).unwrap_or(&default_state);
                let trust = (buyer.trust + 0.01 * step as f64).min(1.0);
                let influence_type = if inf_state.proof_score > 1.0 {
                    Some("social_proof")
                } else if inf_state.skepticism_score > 1.0 {
                    Some("skeptic")
                } else {
                    None
                };
                let reaction = sample_reaction(id, step, trust, &buyer.primary_channel, influence_type, &mut rng);
                transcript.append(TranscriptLine {
                    buyer_id: id.clone(),
                    step,
                    stage: format!("{:?}", buyer.stage),
                    trust,
                    reaction,
                    channel: buyer.primary_channel.clone(),
                    influence_type: influence_type.map(String::from),
                });
            }

            transcript.arc_summary = transcript.build_arc_summary();
            transcript
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arc_summary_trust_building() {
        let buyer_ids: Vec<String> = vec!["b-00000000".to_string()];
        let segment_ids: Vec<String> = vec!["default".to_string()];
        let snapshot = crate::population::SyntheticPopulationSnapshot {
            buyer_count: 1,
            population_seed: 42,
            segment_distribution: std::collections::BTreeMap::new(),
            buyers: vec![crate::buyer::Buyer::default()],
            segment_summaries: std::collections::BTreeMap::new(),
        };
        let influence_states = std::collections::HashMap::new();
        let transcripts = build_transcripts(&buyer_ids, &segment_ids, &snapshot, &influence_states, 3, &TranscriptConfig { sample_rate: 1.0, seed: 7, ..Default::default() });
        assert!(!transcripts.is_empty());
        assert!(!transcripts[0].arc_summary.is_empty());
    }

    #[test]
    fn test_sample_rate_zero() {
        let buyer_ids: Vec<String> = (0..100).map(|i| format!("b{}", i)).collect();
        let segment_ids = vec!["seg".to_string(); 100];
        let snapshot = crate::population::SyntheticPopulationSnapshot {
            buyer_count: 100,
            population_seed: 42,
            segment_distribution: std::collections::BTreeMap::new(),
            buyers: vec![],
            segment_summaries: std::collections::BTreeMap::new(),
        };
        let influence_states = std::collections::HashMap::new();
        let transcripts = build_transcripts(&buyer_ids, &segment_ids, &snapshot, &influence_states, 3, &TranscriptConfig { sample_rate: 0.0, seed: 7, ..Default::default() });
        assert!(transcripts.is_empty());
    }
}
