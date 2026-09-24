use crate::framework::SplitLog;
use crate::{IndexedCut, IndexedEventLog, Lifecycle};
use std::collections::{HashMap, HashSet};

/// Inserts a completion immediately after each unmatched start (§2.3.2).
/// Existing events retain their order and metadata. Synthetic completions copy
/// their start's activity and timestamp. Unmatched completions remain atomic.
/// Returns a sublog sharing the original interner.
pub fn repair_consistency(log: &IndexedEventLog) -> IndexedEventLog {
    let traces = log
        .traces
        .iter()
        .map(|trace| {
            let mut pending: HashMap<_, Vec<usize>> = HashMap::new();
            for (i, e) in trace.events.iter().enumerate() {
                match e.lifecycle {
                    Lifecycle::Start => pending.entry(e.activity).or_default().push(i),
                    Lifecycle::Complete => {
                        if let Some(starts) = pending.get_mut(&e.activity) {
                            starts.pop();
                        }
                    }
                    _ => {}
                }
            }
            let unmatched: HashSet<_> = pending.into_values().flatten().collect();
            let mut repaired = trace.clone();
            repaired.events.clear();
            for (i, e) in trace.events.iter().enumerate() {
                repaired.events.push(e.clone());
                if unmatched.contains(&i) {
                    let mut completion = e.clone();
                    completion.lifecycle = Lifecycle::Complete;
                    repaired.events.push(completion);
                }
            }
            repaired
        })
        .collect();
    log.with_traces(traces)
}

/// Repairs lifecycle consistency in every sublog produced by a splitter.
#[derive(Debug, Clone, Copy, Default)]
pub struct ConsistentSplit<S>(pub S);
impl<S: SplitLog> SplitLog for ConsistentSplit<S> {
    fn split_log(&self, log: &IndexedEventLog, cut: &IndexedCut) -> Vec<IndexedEventLog> {
        self.0
            .split_log(log, cut)
            .iter()
            .map(repair_consistency)
            .collect()
    }
}
