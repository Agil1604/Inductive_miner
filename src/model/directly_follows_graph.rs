use std::collections::HashMap;

use super::{
    Activity,
    event::{EventLog, Lifecycle},
};

/// A frequency-weighted directly-follows abstraction of an event log.
/// Atomic edges record adjacent events; [`Self::from_non_atomic_log`] instead
/// records lifecycle-aware completion-to-start relations.
/// Counts include duplicate traces and repeated events. `A` defaults to named
/// [`Activity`] values and can also be an interned activity ID. Public fields
/// allow filtering and graph construction; their consistency is not validated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectlyFollowsGraph<A: Eq + std::hash::Hash = Activity> {
    /// Event count for atomic construction, execution-start count for non-atomic construction.
    pub activities: HashMap<A, usize>,
    /// Counts of directly-following pairs, including self-loops.
    pub edges: HashMap<(A, A), usize>,
    /// First-event count for atomic logs, qualifying start-event count for non-atomic logs.
    pub start_activities: HashMap<A, usize>,
    /// Last-event count for atomic logs, qualifying completion count for non-atomic logs.
    pub end_activities: HashMap<A, usize>,
    /// Number of recorded traces containing no events.
    pub empty_trace_count: usize,
}

impl<A: Clone + Eq + std::hash::Hash> DirectlyFollowsGraph<A> {
    /// Builds the non-atomic DFG in stored event order.
    ///
    /// A completion of `a` precedes a start of `b` directly when the intervening
    /// events contain no full execution (a start followed by a completion of the
    /// same activity). Start activities have starts before any completion; end
    /// activities have completions after the final start. Multiple activities can
    /// therefore be start or end activities in one trace.
    ///
    /// Each qualifying completion/start pair contributes one edge occurrence;
    /// each start contributes one activity occurrence. Duplicate traces retain
    /// their multiplicity. Unmatched completions and unknown lifecycle events
    /// are represented as instantaneous start/completion pairs, supporting atomic
    /// input. Other transitions are ignored; traces containing only ignored events
    /// count as empty. Timestamps are not consulted. Unmatched starts are not
    /// repaired; callers must validate or normalize inconsistent input separately.
    pub fn from_non_atomic_log(log: &EventLog<A>) -> Self {
        let mut graph = Self::default();
        for trace in &log.traces {
            let mut events = Vec::new();
            let mut active: HashMap<&A, usize> = HashMap::new();
            for event in &trace.events {
                let a = &event.activity;
                match event.lifecycle {
                    Lifecycle::Start => {
                        *active.entry(a).or_default() += 1;
                        events.push((a, true));
                    }
                    Lifecycle::Complete => {
                        let count = active.entry(a).or_default();
                        if *count == 0 {
                            events.push((a, true));
                        } else {
                            *count -= 1;
                        }
                        events.push((a, false));
                    }
                    Lifecycle::Unknown => events.extend([(a, true), (a, false)]),
                    Lifecycle::Enqueue | Lifecycle::Other => {}
                }
            }
            if events.is_empty() {
                graph.empty_trace_count += 1;
                continue;
            }
            let mut completed = false;
            for &(a, start) in &events {
                if start {
                    *graph.activities.entry(a.clone()).or_default() += 1;
                    if !completed {
                        *graph.start_activities.entry(a.clone()).or_default() += 1;
                    }
                } else {
                    completed = true;
                }
            }
            let mut later_start = false;
            for &(a, start) in events.iter().rev() {
                if start {
                    later_start = true;
                } else if !later_start {
                    *graph.end_activities.entry(a.clone()).or_default() += 1;
                }
            }
            for (i, &(a, start)) in events.iter().enumerate() {
                if start {
                    continue;
                }
                let mut interval_starts = std::collections::HashSet::new();
                for &(b, starts) in &events[i + 1..] {
                    if starts {
                        *graph.edges.entry((a.clone(), b.clone())).or_default() += 1;
                        interval_starts.insert(b);
                    } else if interval_starts.contains(b) {
                        break;
                    }
                }
            }
        }
        graph
    }

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

    fn lifecycle_log(events: &[(&str, Lifecycle)]) -> EventLog {
        EventLog {
            traces: vec![Trace {
                case_id: String::new(),
                events: events
                    .iter()
                    .map(|(a, lifecycle)| Event {
                        activity: Activity::from(*a),
                        lifecycle: *lifecycle,
                        timestamp: None,
                    })
                    .collect(),
            }],
        }
    }

    #[test]
    fn non_atomic_thesis_l66() {
        use Lifecycle::{Complete as C, Start as S};
        let log = lifecycle_log(&[
            ("a", S),
            ("a", S),
            ("a", C),
            ("b", S),
            ("b", C),
            ("c", S),
            ("a", C),
            ("c", C),
        ]);
        let graph = DirectlyFollowsGraph::from_non_atomic_log(&log);
        let a = Activity::from("a");
        let b = Activity::from("b");
        let c = Activity::from("c");
        assert_eq!(
            graph.activities,
            HashMap::from([(a.clone(), 2), (b.clone(), 1), (c.clone(), 1)])
        );
        assert_eq!(graph.start_activities, HashMap::from([(a.clone(), 2)]));
        assert_eq!(
            graph.end_activities,
            HashMap::from([(a.clone(), 1), (c.clone(), 1)])
        );
        assert_eq!(
            graph.edges,
            HashMap::from([((a, b.clone()), 1), ((b, c), 1)])
        );
        let indexed = log.indexed();
        let ids = DirectlyFollowsGraph::from_non_atomic_log(&indexed);
        for ((x, y), count) in ids.edges {
            assert_eq!(
                graph
                    .edges
                    .get(&(indexed.resolve(x).clone(), indexed.resolve(y).clone())),
                Some(&count)
            );
        }
    }

    #[test]
    fn nested_overlap_has_multiple_boundaries_and_no_edges() {
        use Lifecycle::{Complete as C, Start as S};
        let graph = DirectlyFollowsGraph::from_non_atomic_log(&lifecycle_log(&[
            ("a", S),
            ("b", S),
            ("b", C),
            ("a", C),
        ]));
        assert!(graph.edges.is_empty());
        assert_eq!(graph.start_activities.len(), 2);
        assert_eq!(graph.end_activities.len(), 2);
    }

    #[test]
    fn atomic_input_matches_atomic_builder_and_ignored_events_are_empty() {
        let input = EventLog {
            traces: vec![trace(&["a", "b", "b"]), trace(&[])],
        };
        assert_eq!(
            DirectlyFollowsGraph::from_non_atomic_log(&input),
            DirectlyFollowsGraph::from_log(&input)
        );
        let input = lifecycle_log(&[("a", Lifecycle::Complete), ("b", Lifecycle::Complete)]);
        assert_eq!(
            DirectlyFollowsGraph::from_non_atomic_log(&input),
            DirectlyFollowsGraph::from_log(&input)
        );
        let graph =
            DirectlyFollowsGraph::from_non_atomic_log(&lifecycle_log(&[("a", Lifecycle::Other)]));
        assert_eq!(graph.empty_trace_count, 1);
        assert!(graph.activities.is_empty());
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
