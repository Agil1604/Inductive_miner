use crate::Trace;
use crate::framework::SplitLog;
use crate::{IndexedCut, IndexedEventLog, OperatorType};

///
/// Splits traces into maximal contiguous runs belonging to each loop part.
/// Returns one sublog per partition in partition order.
///
#[derive(Debug, Clone, Copy, Default)]
pub struct LoopSplit;
impl SplitLog for LoopSplit {
    fn split_log(&self, log: &IndexedEventLog, cut: &IndexedCut) -> Vec<IndexedEventLog> {
        if cut.operator != OperatorType::Loop {
            return Vec::new();
        }
        let mut traces = vec![Vec::new(); cut.partitions.len()];
        for trace in &log.traces {
            let mut start = 0;
            while start < trace.events.len() {
                let Some(i) = cut
                    .partitions
                    .iter()
                    .position(|p| p.contains(&trace.events[start].activity))
                else {
                    return Vec::new();
                };
                let mut end = start + 1;
                while end < trace.events.len()
                    && cut.partitions[i].contains(&trace.events[end].activity)
                {
                    end += 1;
                }
                traces[i].push(Trace {
                    case_id: trace.case_id.clone(),
                    events: trace.events[start..end].to_vec(),
                });
                start = end;
            }
        }
        traces
            .into_iter()
            .map(|traces| log.with_traces(traces))
            .collect()
    }
}
