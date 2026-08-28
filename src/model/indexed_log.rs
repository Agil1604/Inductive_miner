use super::{
    Activity,
    event::{Event, EventLog, Trace},
};
use std::{
    collections::{HashMap, HashSet},
    ops::Deref,
    sync::Arc,
};

///
/// Activity identifier
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ActivityId(usize);
impl ActivityId {
    /// Getter for activity index
    pub fn index(self) -> usize {
        self.0
    }
}

///
/// Immutable name table shared by all sublogs in a mining run.
/// IDs are assigned in lexical order for deterministic traversal.
///
#[derive(Debug, Default)]
pub struct ActivityInterner {
    names: Vec<Activity>,
    ids: HashMap<Activity, ActivityId>,
}
impl ActivityInterner {
    ///
    /// Looks up an existing name without adding an activity.
    ///
    pub fn id(&self, activity: &Activity) -> Option<ActivityId> {
        self.ids.get(activity).copied()
    }
    ///
    /// Resolves an ID. IDs from another interner must not be used here.
    ///
    pub fn resolve(&self, id: ActivityId) -> Option<&Activity> {
        self.names.get(id.index())
    }
    pub fn len(&self) -> usize {
        self.names.len()
    }
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

///
/// Encoded log retaining metadata and sharing its immutable name table.
///
#[derive(Debug, Clone)]
pub struct IndexedEventLog {
    log: EventLog<ActivityId>,
    interner: Arc<ActivityInterner>,
}

impl IndexedEventLog {
    ///
    /// Encodes activity names once, preserving trace order and metadata.
    ///
    pub fn from_log(log: &EventLog) -> Self {
        let names = log.alphabet();
        let ids: HashMap<_, _> = names
            .iter()
            .enumerate()
            .map(|(i, a)| {
                (
                    a.clone(),
                    ActivityId(
                        usize::try_from(i).expect("activity alphabet exceeds usize capacity"),
                    ),
                )
            })
            .collect();
        let traces = log
            .traces
            .iter()
            .map(|t| Trace {
                case_id: t.case_id.clone(),
                events: t
                    .events
                    .iter()
                    .map(|e| Event {
                        activity: ids[&e.activity],
                        lifecycle: e.lifecycle,
                        timestamp: e.timestamp,
                    })
                    .collect(),
            })
            .collect();
        Self {
            log: EventLog { traces },
            interner: Arc::new(ActivityInterner { names, ids }),
        }
    }

    ///
    /// Creates a sublog using this log's name table.
    ///
    pub fn with_traces(&self, traces: Vec<Trace<ActivityId>>) -> Self {
        Self {
            log: EventLog { traces },
            interner: Arc::clone(&self.interner),
        }
    }

    pub(crate) fn log_mut_traces(&mut self) -> &mut Vec<Trace<ActivityId>> {
        &mut self.log.traces
    }

    pub fn interner(&self) -> &ActivityInterner {
        &self.interner
    }

    ///
    /// Resolves a local ID to its original activity name.
    ///
    pub fn resolve(&self, id: ActivityId) -> &Activity {
        self.interner
            .resolve(id)
            .expect("activity ID outside this log's interner")
    }

    ///
    /// Checks identity of the shared mapping, not just equal numeric IDs.
    ///
    pub fn shares_interner(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.interner, &other.interner)
    }

    pub fn activities(&self) -> HashSet<ActivityId> {
        self.log.activities()
    }

    pub fn alphabet(&self) -> Vec<ActivityId> {
        self.log.alphabet()
    }
}

impl Deref for IndexedEventLog {
    type Target = EventLog<ActivityId>;
    fn deref(&self) -> &Self::Target {
        &self.log
    }
}

impl EventLog {
    ///
    /// Encodes this log for mining; recursive sublogs share the resulting table.
    ///
    pub fn indexed(&self) -> IndexedEventLog {
        IndexedEventLog::from_log(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::log;
    use crate::{IndexedDfg, Lifecycle};

    #[test]
    fn deduplicates_names_and_assigns_lexical_ids() {
        let input = log(&[&["z", "a", "z"], &["a"], &[]]);
        let encoded = input.indexed();
        assert_eq!(encoded.interner().len(), 2);
        assert_eq!(
            encoded.interner().id(&Activity::from("a")).unwrap().index(),
            0
        );
        assert_eq!(
            encoded.interner().id(&Activity::from("z")).unwrap().index(),
            1
        );
        assert_eq!(
            encoded.traces[0].events[0].activity,
            encoded.traces[0].events[2].activity
        );
        assert!(encoded.interner().id(&Activity::from("missing")).is_none());
        assert!(encoded.traces[2].events.is_empty());
        for (original, indexed) in input.traces.iter().zip(&encoded.traces) {
            assert_eq!(original.case_id, indexed.case_id);
            for (e, i) in original.events.iter().zip(&indexed.events) {
                assert_eq!(&e.activity, encoded.resolve(i.activity));
            }
        }
    }

    #[test]
    fn sublogs_share_mapping_even_when_their_alphabet_is_smaller() {
        let encoded = log(&[&["a", "b"], &["b"]]).indexed();
        let sublog = encoded.with_traces(vec![encoded.traces[1].clone()]);
        assert!(encoded.shares_interner(&sublog));
        let b = sublog.traces[0].events[0].activity;
        assert_eq!(b.index(), 1);
        assert_eq!(sublog.resolve(b), &Activity::from("b"));
        let independently_encoded = log(&[&["a", "b"]]).indexed();
        assert!(!encoded.shares_interner(&independently_encoded));
    }

    #[test]
    fn encoding_preserves_timestamps_and_lifecycle() {
        let mut input = log(&[&["a"]]);
        let timestamp = chrono::DateTime::parse_from_rfc3339("2026-10-03T12:00:00+03:00")
            .unwrap()
            .with_timezone(&chrono::Utc);
        input.traces[0].events[0].lifecycle = Lifecycle::Start;
        input.traces[0].events[0].timestamp = Some(timestamp);
        let encoded = input.indexed();
        assert_eq!(encoded.traces[0].events[0].timestamp, Some(timestamp));
        assert_eq!(encoded.traces[0].events[0].lifecycle, Lifecycle::Start);
    }

    #[test]
    fn indexed_graph_counts_are_equivalent_to_named_graph() {
        let input = log(&[&["b", "b", "a"], &["a"], &[]]);
        let encoded = input.indexed();
        let graph = IndexedDfg::from_log(&encoded);
        let named = crate::DirectlyFollowsGraph::from_log(&input);
        assert_eq!(graph.empty_trace_count, named.empty_trace_count);
        for (id, count) in graph.activities {
            assert_eq!(named.activities[encoded.resolve(id)], count);
        }
        for ((a, b), count) in graph.edges {
            assert_eq!(
                named.edges[&(encoded.resolve(a).clone(), encoded.resolve(b).clone())],
                count
            );
        }
    }
}
