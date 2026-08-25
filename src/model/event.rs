use chrono::{DateTime, Utc};
use std::collections::HashSet;

use super::Activity;

///
/// Represents a lifecycle transition of an activity in a process model
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lifecycle {
    Start,
    Complete,
    Enqueue,
    /// Unrecognized lifecycle transition.
    Other,
    /// Lifecycle was not recorded.
    Unknown,
}

impl Lifecycle {
    /// Create a [`Lifecycle`] from a string representation in XES
    pub fn from_xes(s: &str) -> Self {
        match s {
            "start" => Lifecycle::Start,
            "complete" => Lifecycle::Complete,
            "enqueue" => Lifecycle::Enqueue,
            _ => Lifecycle::Other,
        }
    }
}

///
/// Represents an event in a process model
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event<A = Activity> {
    pub activity: A,
    pub lifecycle: Lifecycle,
    pub timestamp: Option<DateTime<Utc>>,
}

///
/// Represents a trace in a process model
///
#[derive(Debug, Clone)]
pub struct Trace<A = Activity> {
    pub case_id: String,
    pub events: Vec<Event<A>>,
}

///
/// Represents an event log in a process model
///
#[derive(Debug, Clone)]
pub struct EventLog<A = Activity> {
    pub traces: Vec<Trace<A>>,
}

///
/// Represents an event classifier in a process model
///
#[derive(Debug, Clone)]
pub struct EventClassifier {
    pub activity_key: String,
    pub timestamp_key: Option<String>,
    pub lifecycle_key: Option<String>,
}

impl Default for EventClassifier {
    fn default() -> Self {
        Self {
            activity_key: "concept:name".to_string(),
            timestamp_key: Some("time:timestamp".to_string()),
            lifecycle_key: Some("lifecycle:transition".to_string()),
        }
    }
}
impl<A: Clone + Eq + std::hash::Hash + Ord> EventLog<A> {
    ///
    /// Returns distinct activities in lexical order of their names.
    /// Event order, multiplicities, and metadata do not affect the result.
    ///
    pub fn alphabet(&self) -> Vec<A> {
        let mut activities: Vec<_> = self.activities().into_iter().collect();
        activities.sort();
        activities
    }

    ///
    /// Activity alphabet, independent of case identifiers and timestamps.
    ///
    pub fn activities(&self) -> HashSet<A> {
        self.traces
            .iter()
            .flat_map(|t| &t.events)
            .map(|e| e.activity.clone())
            .collect()
    }

    ///
    /// Project recorded lifecycle events to atomic executions. Unannotated events
    /// are assumed atomic; explicitly annotated non-completion events are omitted.
    /// Trace order, duplicate traces, and empty traces are preserved.
    ///
    pub fn atomic(&self) -> Self {
        Self {
            traces: self
                .traces
                .iter()
                .map(|t| Trace {
                    case_id: t.case_id.clone(),
                    events: t
                        .events
                        .iter()
                        .filter(|e| matches!(e.lifecycle, Lifecycle::Complete | Lifecycle::Unknown))
                        .cloned()
                        .collect(),
                })
                .collect(),
        }
    }
}

impl<A> Default for Trace<A> {
    fn default() -> Self {
        Self {
            case_id: String::new(),
            events: Vec::new(),
        }
    }
}

impl<A> Default for EventLog<A> {
    fn default() -> Self {
        Self { traces: Vec::new() }
    }
}
