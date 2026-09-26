use chrono::{DateTime, Utc};
use std::collections::HashSet;

use super::Activity;

/// The recorded stage of an activity execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lifecycle {
    /// Execution begins.
    Start,
    /// Execution finishes.
    Complete,
    /// Execution is queued.
    Enqueue,
    /// Unrecognized lifecycle transition.
    Other,
    /// Lifecycle was not recorded.
    Unknown,
}

impl Lifecycle {
    /// Maps the exact XES values `start`, `complete`, and `enqueue` to variants.
    ///
    /// All other values, including an empty string or different capitalization,
    /// become [`Self::Other`]. Missing attributes are handled separately as
    /// [`Self::Unknown`].
    pub fn from_xes(s: &str) -> Self {
        match s {
            "start" => Lifecycle::Start,
            "complete" => Lifecycle::Complete,
            "enqueue" => Lifecycle::Enqueue,
            _ => Lifecycle::Other,
        }
    }
}

/// One recorded event with an activity label and optional time information.
///
/// `A` defaults to [`Activity`]; indexed logs use [`super::indexed_log::ActivityId`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event<A = Activity> {
    /// Named activity or interned identifier associated with this event.
    pub activity: A,
    /// Recorded execution stage, or [`Lifecycle::Unknown`] if absent.
    pub lifecycle: Lifecycle,
    /// Recorded time in UTC, or `None` when unavailable.
    pub timestamp: Option<DateTime<Utc>>,
}

/// An ordered sequence of events for one case, possibly empty.
///
/// Events are stored in execution/log order; they are not automatically sorted
/// by timestamp. `A` has the same meaning as in [`Event`].
#[derive(Debug, Clone)]
pub struct Trace<A = Activity> {
    /// Case identifier. It may be empty and is not required to be unique.
    pub case_id: String,
    /// Events belonging to the case in their recorded order.
    pub events: Vec<Event<A>>,
}

/// A collection of ordered traces, retaining duplicate cases and empty traces.
///
/// `A` defaults to named [`Activity`] values and can also represent interned IDs.
/// An empty log contains no traces; a log with an empty trace still records a case.
///
/// # Examples
/// ```
/// use inductive_miner::{
///     Activity, Event, EventLog, Trace, Lifecycle, DirectlyFollowsGraph,
/// };
///
/// let log = EventLog {
///     traces: vec![Trace {
///         case_id: "case-1".into(),
///         events: vec![Event {
///             activity: Activity::from("A"),
///             lifecycle: Lifecycle::Complete,
///             timestamp: None,
///         }],
///     }],
/// };
/// assert_eq!(log.alphabet(), vec![Activity::from("A")]);
/// let graph = DirectlyFollowsGraph::from_log(&log);
/// assert_eq!(graph.activities[&Activity::from("A")], 1);
/// let indexed = log.indexed();
/// let id = indexed.traces[0].events[0].activity;
/// assert_eq!(indexed.resolve(id), &Activity::from("A"));
/// let sublog = indexed.with_traces(vec![indexed.traces[0].clone()]);
/// assert!(indexed.shares_interner(&sublog));
/// ```
#[derive(Debug, Clone)]
pub struct EventLog<A = Activity> {
    /// Recorded traces in input order, including duplicates.
    pub traces: Vec<Trace<A>>,
}

/// Attribute keys used by the XES reader to populate event fields.
///
/// This selects individual keys rather than composing a multi-key XES classifier.
#[derive(Debug, Clone)]
pub struct EventClassifier {
    /// Required activity attribute key, defaulting to `concept:name`.
    pub activity_key: String,
    /// Optional time attribute key, defaulting to `Some("time:timestamp")`.
    /// `None` disables timestamp extraction.
    pub timestamp_key: Option<String>,
    /// Optional lifecycle key, defaulting to `Some("lifecycle:transition")`.
    /// `None` leaves lifecycle values unknown.
    pub lifecycle_key: Option<String>,
}

impl Default for EventClassifier {
    /// Selects the standard XES activity, timestamp, and lifecycle keys.
    fn default() -> Self {
        Self {
            activity_key: "concept:name".to_string(),
            timestamp_key: Some("time:timestamp".to_string()),
            lifecycle_key: Some("lifecycle:transition".to_string()),
        }
    }
}
impl<A: Clone + Eq + std::hash::Hash + Ord> EventLog<A> {
    /// Returns distinct activities sorted by `A`'s ordering.
    /// Named activities sort lexically; interned IDs sort by numeric index.
    /// Event order, multiplicities, and metadata do not affect the result.
    pub fn alphabet(&self) -> Vec<A> {
        let mut activities: Vec<_> = self.activities().into_iter().collect();
        activities.sort();
        activities
    }

    /// Activity alphabet, independent of case identifiers and timestamps.
    pub fn activities(&self) -> HashSet<A> {
        self.traces
            .iter()
            .flat_map(|t| &t.events)
            .map(|e| e.activity.clone())
            .collect()
    }

    /// Project recorded lifecycle events to atomic executions. Unannotated events
    /// are assumed atomic; explicitly annotated non-completion events are omitted.
    /// Trace order, duplicate traces, and empty traces are preserved.
    /// Retained events keep their metadata; traces may become empty. This is a
    /// projection, not a start/complete pairing algorithm.
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
    /// Creates an empty trace with an empty case identifier.
    fn default() -> Self {
        Self {
            case_id: String::new(),
            events: Vec::new(),
        }
    }
}

impl<A> Default for EventLog<A> {
    /// Creates a log containing no traces.
    fn default() -> Self {
        Self { traces: Vec::new() }
    }
}
