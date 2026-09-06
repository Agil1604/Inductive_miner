use std::collections::HashMap;

use super::{Activity, event::EventLog};

/// A frequency-weighted directly-follows abstraction of an event log.
/// An edge `(a, b)` records how often `b` immediately follows `a` within a trace.
/// Counts include duplicate traces and repeated events. `A` defaults to named
/// [`Activity`] values and can also be an interned activity ID. Public fields
/// allow filtering and graph construction; their consistency is not validated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectlyFollowsGraph<A: Eq + std::hash::Hash = Activity> {
    /// Total event occurrence count per activity, including isolated vertices.
    pub activities: HashMap<A, usize>,
    /// Counts of adjacent ordered activity pairs, including self-loops.
    pub edges: HashMap<(A, A), usize>,
    /// Number of nonempty traces starting with each activity.
    pub start_activities: HashMap<A, usize>,
    /// Number of nonempty traces ending with each activity.
    pub end_activities: HashMap<A, usize>,
    /// Number of recorded traces containing no events.
    pub empty_trace_count: usize,
}

impl<A: Clone + Eq + std::hash::Hash> DirectlyFollowsGraph<A> {
    /// Builds a graph from events in their stored trace order.
    ///
    /// Does not sort timestamps or filter lifecycle transitions. Empty traces
    /// contribute only to [`Self::empty_trace_count`]; single-event traces
    /// contribute an activity and both boundary counts, but no edge.
    pub fn from_log(log: &EventLog<A>) -> Self {
        let mut graph = Self {
            activities: HashMap::new(),
            edges: HashMap::new(),
            start_activities: HashMap::new(),
            end_activities: HashMap::new(),
            empty_trace_count: 0,
        };
        for trace in &log.traces {
            let Some(first) = trace.events.first() else {
                graph.empty_trace_count += 1;
                continue;
            };
            *graph
                .start_activities
                .entry(first.activity.clone())
                .or_default() += 1;

            let last = trace.events.last().unwrap();
            *graph
                .end_activities
                .entry(last.activity.clone())
                .or_default() += 1;
            for event in &trace.events {
                *graph.activities.entry(event.activity.clone()).or_default() += 1;
            }
            for pair in trace.events.windows(2) {
                *graph
                    .edges
                    .entry((pair[0].activity.clone(), pair[1].activity.clone()))
                    .or_default() += 1;
            }
        }
        graph
    }
}

impl<A: Clone + Eq + std::hash::Hash> From<&EventLog<A>> for DirectlyFollowsGraph<A> {
    /// Constructs a graph using [`Self::from_log`].
    fn from(log: &EventLog<A>) -> Self {
        Self::from_log(log)
    }
}

impl<A: Eq + std::hash::Hash> Default for DirectlyFollowsGraph<A> {
    /// Creates a graph with no activities, edges, or recorded empty traces.
    fn default() -> Self {
        Self {
            activities: HashMap::new(),
            edges: HashMap::new(),
            start_activities: HashMap::new(),
            end_activities: HashMap::new(),
            empty_trace_count: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Event, Lifecycle, Trace};

    fn trace(names: &[&str]) -> Trace {
        Trace {
            case_id: String::new(),
            events: names
                .iter()
                .map(|name| Event {
                    activity: Activity::from(*name),
                    lifecycle: Lifecycle::Unknown,
                    timestamp: None,
                })
                .collect(),
        }
    }

    #[test]
    fn counts_duplicates_self_loops_boundaries_and_isolated_vertices() {
        let log = EventLog {
            traces: vec![
                trace(&["a", "a", "b"]),
                trace(&["a", "a", "b"]),
                trace(&["c"]),
                trace(&[]),
                trace(&[]),
            ],
        };
        let graph = DirectlyFollowsGraph::from(&log);
        println!("{:?}", graph);
        assert_eq!(
            graph.activities,
            HashMap::from([
                (Activity::from("a"), 4),
                (Activity::from("b"), 2),
                (Activity::from("c"), 1),
            ])
        );
        assert_eq!(
            graph.edges,
            HashMap::from([
                ((Activity::from("a"), Activity::from("a")), 2),
                ((Activity::from("a"), Activity::from("b")), 2),
            ])
        );
        assert_eq!(
            graph.start_activities,
            HashMap::from([(Activity::from("a"), 2), (Activity::from("c"), 1)])
        );
        assert_eq!(
            graph.end_activities,
            HashMap::from([(Activity::from("b"), 2), (Activity::from("c"), 1)])
        );
        assert_eq!(graph.empty_trace_count, 2);
    }

    #[test]
    fn distinguishes_empty_log_from_empty_trace() {
        assert_eq!(
            DirectlyFollowsGraph::<Activity>::from_log(&EventLog::default()),
            DirectlyFollowsGraph::default()
        );
        let graph = DirectlyFollowsGraph::from_log(&EventLog {
            traces: vec![trace(&[])],
        });
        assert!(graph.activities.is_empty());
        assert!(graph.edges.is_empty());
        assert!(graph.start_activities.is_empty());
        assert!(graph.end_activities.is_empty());
        assert_eq!(graph.empty_trace_count, 1);
    }
}
