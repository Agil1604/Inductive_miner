use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use super::{
    Activity,
    event::{EventLog, Lifecycle},
};

/// Observed execution overlap.
///
/// Edges are undirected and stored once, with the smaller activity first.
/// Self-edges represent overlapping executions of the same activity.
/// Named activities and interned IDs are both supported. Public fields are
/// not validated when modified by callers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConcurrencyGraph<A: Eq + Hash = Activity> {
    /// Activities with a start, completion, or unannotated event.
    pub activities: HashSet<A>,
    /// Distinct observed overlaps, including self-overlap.
    pub edges: HashSet<(A, A)>,
    /// Maximum number of simultaneous executions of each activity in any trace.
    pub max_concurrency: HashMap<A, usize>,
}

impl<A: Clone + Eq + Hash + Ord> ConcurrencyGraph<A> {
    /// Builds the graph in stored event order, without consulting timestamps.
    ///
    /// Starts open executions; completions close one execution of their activity.
    /// Unmatched completions and [`Lifecycle::Unknown`] events are treated as
    /// instantaneous atomic executions, which can overlap open executions.
    /// Enqueue and other transitions are ignored. Execution counts reset for
    /// every trace, and duplicate traces do not duplicate edges.
    ///
    /// Consistent lifecycle traces need no explicit start/completion mapping.
    /// Unmatched starts remain open until the end of their trace; this function
    /// does not validate or repair inconsistent input.
    /// 
    /// # Preconditions
    ///
    /// Events must be in chronological order within each trace.
    /// If events are stored out of order sort them first.
    pub fn from_log(log: &EventLog<A>) -> Self {
        let mut graph = Self::default();
        for trace in &log.traces {
            let mut active: HashMap<A, usize> = HashMap::new();
            for event in &trace.events {
                let activity = &event.activity;
                let opens = match event.lifecycle {
                    Lifecycle::Start => true,
                    Lifecycle::Complete => active.get(activity).copied().unwrap_or(0) == 0,
                    Lifecycle::Unknown => true,
                    Lifecycle::Enqueue | Lifecycle::Other => continue,
                };
                graph.activities.insert(activity.clone());
                if opens {
                    for other in active.keys() {
                        let edge = if activity <= other {
                            (activity.clone(), other.clone())
                        } else {
                            (other.clone(), activity.clone())
                        };
                        graph.edges.insert(edge);
                    }
                    let simultaneous = active.get(activity).copied().unwrap_or(0) + 1;
                    let maximum = graph.max_concurrency.entry(activity.clone()).or_default();
                    *maximum = (*maximum).max(simultaneous);
                }
                match event.lifecycle {
                    Lifecycle::Start => *active.entry(activity.clone()).or_default() += 1,
                    Lifecycle::Complete => {
                        if let Some(count) = active.get_mut(activity) {
                            *count -= 1;
                            if *count == 0 {
                                active.remove(activity);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        graph
    }

    /// Reports whether the two activities overlap, regardless of argument order.
    pub fn is_concurrent(&self, a: &A, b: &A) -> bool {
        let pair = if a <= b {
            (a.clone(), b.clone())
        } else {
            (b.clone(), a.clone())
        };
        self.edges.contains(&pair)
    }
}

impl<A: Clone + Eq + Hash + Ord> From<&EventLog<A>> for ConcurrencyGraph<A> {
    /// Constructs the graph using [`Self::from_log`].
    fn from(log: &EventLog<A>) -> Self {
        Self::from_log(log)
    }
}

impl<A: Eq + Hash> Default for ConcurrencyGraph<A> {
    /// Creates a graph without activities or overlaps.
    fn default() -> Self {
        Self {
            activities: HashSet::new(),
            edges: HashSet::new(),
            max_concurrency: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Event, Trace};
    use Lifecycle::{Complete as C, Start as S};

    fn log(traces: &[&[(&str, Lifecycle)]]) -> EventLog {
        EventLog {
            traces: traces
                .iter()
                .map(|events| Trace {
                    case_id: String::new(),
                    events: events
                        .iter()
                        .map(|(name, lifecycle)| Event {
                            activity: Activity::from(*name),
                            lifecycle: *lifecycle,
                            timestamp: None,
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    #[test]
    fn thesis_l66_overlap_and_self_overlap() {
        let input = log(&[&[
            ("a", S),
            ("a", S),
            ("a", C),
            ("b", S),
            ("b", C),
            ("c", S),
            ("a", C),
            ("c", C),
        ]]);
        let graph = ConcurrencyGraph::from(&input);
        let a = Activity::from("a");
        let b = Activity::from("b");
        let c = Activity::from("c");
        assert_eq!(
            graph.edges,
            HashSet::from([
                (a.clone(), a.clone()),
                (a.clone(), b.clone()),
                (a.clone(), c.clone())
            ])
        );
        assert!(graph.is_concurrent(&b, &a));
        assert!(!graph.is_concurrent(&b, &c));
        assert_eq!(
            graph.max_concurrency,
            HashMap::from([(a, 2), (b, 1), (c, 1)])
        );
        let indexed = input.indexed();
        let indexed_graph = ConcurrencyGraph::from_log(&indexed);
        for (x, y) in indexed_graph.edges {
            assert!(graph.is_concurrent(indexed.resolve(x), indexed.resolve(y)));
        }
    }

    #[test]
    fn sequential_executions_and_separate_traces_do_not_overlap() {
        let graph = ConcurrencyGraph::from_log(&log(&[
            &[("a", S), ("a", C), ("b", S), ("b", C)],
            &[("a", S)],
            &[("b", S), ("b", C)],
            &[],
        ]));
        assert!(graph.edges.is_empty());
        assert_eq!(graph.activities.len(), 2);
    }

    #[test]
    fn atomic_events_overlap_open_executions_and_other_transitions_are_ignored() {
        let graph = ConcurrencyGraph::from_log(&log(&[&[
            ("a", S),
            ("b", C),
            ("c", Lifecycle::Unknown),
            ("ignored", Lifecycle::Enqueue),
            ("ignored", Lifecycle::Other),
            ("a", C),
        ]]));
        assert_eq!(graph.edges.len(), 2);
        assert_eq!(graph.activities.len(), 3);
        assert!(!graph.is_concurrent(&Activity::from("b"), &Activity::from("c")));
        assert!(graph.max_concurrency.values().all(|count| *count == 1));
    }

    #[test]
    fn empty_logs_and_duplicate_traces() {
        assert_eq!(
            ConcurrencyGraph::<Activity>::from_log(&EventLog::default()),
            ConcurrencyGraph::default()
        );
        let events = [("a", S), ("b", S), ("a", C), ("b", C)];
        assert_eq!(
            ConcurrencyGraph::from_log(&log(&[&events])),
            ConcurrencyGraph::from_log(&log(&[&events, &events, &[]]))
        );
    }
}
