use std::collections::HashMap;

use super::{Activity, event::EventLog};

///
/// A frequency-weighted directly-follows abstraction of an event log.
/// An edge `(a, b)` records how often `b` immediately follows `a` within a trace.
///
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DirectlyFollowsGraph {
    pub activities: HashMap<Activity, usize>,
    pub edges: HashMap<(Activity, Activity), usize>,
    pub start_activities: HashMap<Activity, usize>,
    pub end_activities: HashMap<Activity, usize>,
    pub empty_trace_count: usize,
}

impl DirectlyFollowsGraph {
    ///
    /// Builds a graph from events in their stored trace order.
    ///
    pub fn from_log(log: &EventLog) -> Self {
        let mut graph = Self::default();
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

impl From<&EventLog> for DirectlyFollowsGraph {
    fn from(log: &EventLog) -> Self {
        Self::from_log(log)
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
            DirectlyFollowsGraph::from_log(&EventLog::default()),
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
