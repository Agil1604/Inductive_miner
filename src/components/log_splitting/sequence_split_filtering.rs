use crate::framework::SplitLog;
use crate::{IndexedCut, IndexedEventLog, OperatorType, Trace};
use std::collections::HashSet;

///
/// Sequence-split filtering strategy for splitting logs according to a cut.
///
#[derive(Debug, Clone, Copy, Default)]
pub struct SequenceSplitFiltering;
impl SplitLog for SequenceSplitFiltering {
    fn split_log(&self, log: &IndexedEventLog, cut: &IndexedCut) -> Vec<IndexedEventLog> {
        if cut.operator != OperatorType::Sequence {
            return Vec::new();
        }
        let mut traces = vec![Vec::new(); cut.partitions.len()];
        for trace in &log.traces {
            let mut start = 0;
            let mut ignored = HashSet::new();
            for (i, part) in cut.partitions.iter().enumerate() {
                let mut cost = 0isize;
                let mut least = 0isize;
                let mut end = start;
                for (position, event) in trace.events.iter().enumerate().skip(start) {
                    if part.contains(&event.activity) {
                        cost -= 1;
                    } else if !ignored.contains(&event.activity) {
                        cost += 1;
                    }
                    if cost < least {
                        least = cost;
                        end = position + 1;
                    }
                }
                traces[i].push(Trace {
                    case_id: trace.case_id.clone(),
                    events: trace.events[start..end]
                        .iter()
                        .filter(|e| part.contains(&e.activity))
                        .cloned()
                        .collect(),
                });
                start = end;
                ignored.extend(part.iter().copied());
            }
        }
        traces.into_iter().map(|t| log.with_traces(t)).collect()
    }
}
