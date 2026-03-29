//! Event-sourced replay: deterministic, replayable simulation runs.
//!
//! Pattern extracted from `wargame-engine`: every simulation run produces
//! an immutable log of events and state snapshots that can be replayed
//! for debugging, auditing, or visualization.

use serde::{Deserialize, Serialize};

use crate::state::SimState;

/// A complete replay of a simulation run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayRun {
    /// Unique identifier for this run.
    pub run_id: String,
    /// Seed used for deterministic replay.
    pub seed: u64,
    /// Final state at end of run.
    pub final_state: SimState,
    /// State snapshots at each time step.
    pub state_snapshots: Vec<StateSnapshot>,
    /// Ordered event log.
    pub event_log: EventLog,
}

/// Snapshot of the full state at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub t: usize,
    pub state: SimState,
    pub health_index: f64,
}

/// Ordered log of events during a simulation run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventLog {
    pub entries: Vec<EventEntry>,
}

impl EventLog {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn push(&mut self, entry: EventEntry) {
        self.entries.push(entry);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Filter events by kind.
    pub fn filter_kind(&self, kind: &EventKind) -> Vec<&EventEntry> {
        self.entries
            .iter()
            .filter(|e| std::mem::discriminant(&e.kind) == std::mem::discriminant(kind))
            .collect()
    }
}

impl Default for EventLog {
    fn default() -> Self {
        Self::new()
    }
}

/// A single event in the simulation log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEntry {
    /// Monotonically increasing sequence number.
    pub sequence: u64,
    /// Time step when this event occurred.
    pub t: usize,
    /// What happened.
    pub kind: EventKind,
    /// Optional metadata.
    pub metadata: Option<serde_json::Value>,
}

/// Types of events that can occur during simulation.
/// Domain-agnostic; consumers can use `Custom(String)` for domain-specific events.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EventKind {
    /// Simulation step started.
    StepStarted,
    /// An action was applied.
    ActionApplied,
    /// State transition occurred (with delta info).
    StateTransition,
    /// Threshold crossed (break point, recovery, etc.)
    ThresholdCrossed { dimension: usize, direction: String },
    /// Simulation step completed.
    StepCompleted,
    /// Run started.
    RunStarted,
    /// Run completed.
    RunCompleted,
    /// Domain-specific event.
    Custom(String),
}

/// Builder for constructing replay runs during simulation.
pub struct ReplayBuilder {
    run_id: String,
    seed: u64,
    snapshots: Vec<StateSnapshot>,
    log: EventLog,
    sequence: u64,
}

impl ReplayBuilder {
    pub fn new(run_id: impl Into<String>, seed: u64) -> Self {
        let mut builder = Self {
            run_id: run_id.into(),
            seed,
            snapshots: Vec::new(),
            log: EventLog::new(),
            sequence: 0,
        };
        builder.emit(0, EventKind::RunStarted, None);
        builder
    }

    /// Record a state snapshot.
    pub fn snapshot(&mut self, state: &SimState, health_index: f64) {
        self.snapshots.push(StateSnapshot {
            t: state.t,
            state: state.clone(),
            health_index,
        });
    }

    /// Emit an event.
    pub fn emit(&mut self, t: usize, kind: EventKind, metadata: Option<serde_json::Value>) {
        self.log.push(EventEntry {
            sequence: self.sequence,
            t,
            kind,
            metadata,
        });
        self.sequence += 1;
    }

    /// Finalize into a `ReplayRun`.
    pub fn finish(mut self, final_state: SimState) -> ReplayRun {
        let t = final_state.t;
        self.emit(t, EventKind::RunCompleted, None);
        ReplayRun {
            run_id: self.run_id,
            seed: self.seed,
            final_state,
            state_snapshots: self.snapshots,
            event_log: self.log,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replay_builder() {
        let mut builder = ReplayBuilder::new("test-run-1", 42);

        let state = SimState::new(vec![0.5], vec![0.0], vec![0.5]);
        builder.snapshot(&state, 0.5);
        builder.emit(0, EventKind::ActionApplied, None);

        let replay = builder.finish(state);

        assert_eq!(replay.run_id, "test-run-1");
        assert_eq!(replay.seed, 42);
        assert_eq!(replay.state_snapshots.len(), 1);
        // RunStarted + ActionApplied + RunCompleted = 3
        assert_eq!(replay.event_log.len(), 3);
    }

    #[test]
    fn test_event_filter() {
        let mut log = EventLog::new();
        log.push(EventEntry { sequence: 0, t: 0, kind: EventKind::RunStarted, metadata: None });
        log.push(EventEntry { sequence: 1, t: 0, kind: EventKind::ActionApplied, metadata: None });
        log.push(EventEntry { sequence: 2, t: 1, kind: EventKind::ActionApplied, metadata: None });
        log.push(EventEntry { sequence: 3, t: 1, kind: EventKind::RunCompleted, metadata: None });

        let actions = log.filter_kind(&EventKind::ActionApplied);
        assert_eq!(actions.len(), 2);
    }
}
