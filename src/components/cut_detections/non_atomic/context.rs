use super::super::CutContext;
use crate::{ActivityId, IndexedConcurrencyGraph, IndexedDfg, IndexedEventLog, Lifecycle};
use std::cell::OnceCell;

/// Shared lifecycle abstractions for IMlc cut detection (§6.5.2).
#[derive(Debug)]
pub struct NonAtomicCutContext<'a> {
    /// Original indexed log, with lifecycle events retained.
    pub log: &'a IndexedEventLog,
    /// Alphabet of the lifecycle-aware graph.
    pub activities: Vec<ActivityId>,
    /// Completion-to-start directly-follows graph.
    pub dfg: IndexedDfg,
    /// Observed execution overlaps.
    pub concurrency: IndexedConcurrencyGraph,
    dfg_context: OnceCell<CutContext<'a>>,
}
impl<'a> NonAtomicCutContext<'a> {
    /// Builds both graphs once; rejects effective empty traces and small alphabets.
    /// Unknown events are atomic; enqueue and other transitions are ignored.
    pub fn new(log: &'a IndexedEventLog) -> Option<Self> {
        if log.traces.iter().any(|t| {
            !t.events.iter().any(|e| {
                matches!(
                    e.lifecycle,
                    Lifecycle::Start | Lifecycle::Complete | Lifecycle::Unknown
                )
            })
        }) {
            return None;
        }
        let dfg = IndexedDfg::from_non_atomic_log(log);
        let mut activities: Vec<_> = dfg.activities.keys().copied().collect();
        activities.sort();
        if activities.len() < 2 {
            return None;
        }
        Some(Self {
            log,
            activities,
            dfg,
            concurrency: IndexedConcurrencyGraph::from_log(log),
            dfg_context: OnceCell::new(),
        })
    }
    pub(super) fn dfg_context(&self) -> &CutContext<'a> {
        self.dfg_context.get_or_init(|| {
            let mut graph = self.dfg.clone();
            for &(a, b) in &self.concurrency.edges {
                graph.edges.entry((a, b)).or_insert(1);
                graph.edges.entry((b, a)).or_insert(1);
            }
            CutContext {
                log: self.log,
                activities: self.activities.clone(),
                dfg: graph,
                witnesses: OnceCell::new(),
            }
        })
    }
    pub(super) fn edge(&self, a: ActivityId, b: ActivityId) -> bool {
        self.dfg.edges.contains_key(&(a, b))
    }
    pub(super) fn overlap(&self, a: ActivityId, b: ActivityId) -> bool {
        self.concurrency.is_concurrent(&a, &b)
    }
}
