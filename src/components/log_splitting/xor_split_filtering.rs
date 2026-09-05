use crate::framework::SplitLog;
use crate::{IndexedCut, IndexedEventLog, OperatorType, Trace};

///
/// XOR-split filtering strategy for splitting logs according to a cut.
///
#[derive(Debug, Clone, Copy, Default)]
pub struct XorSplitFiltering;
impl SplitLog for XorSplitFiltering {
    fn split_log(&self, log: &IndexedEventLog, cut: &IndexedCut) -> Vec<IndexedEventLog> {
        if cut.operator != OperatorType::Xor || cut.partitions.is_empty() {
            return Vec::new();
        }
        let mut traces = vec![Vec::new(); cut.partitions.len()];
        for trace in &log.traces {
            let mut chosen = 0;
            let mut most = 0;
            for (i, part) in cut.partitions.iter().enumerate() {
                let count = trace
                    .events
                    .iter()
                    .filter(|e| part.contains(&e.activity))
                    .count();
                if count > most {
                    chosen = i;
                    most = count;
                }
            }
            traces[chosen].push(Trace {
                case_id: trace.case_id.clone(),
                events: trace
                    .events
                    .iter()
                    .filter(|e| cut.partitions[chosen].contains(&e.activity))
                    .cloned()
                    .collect(),
            });
        }
        traces.into_iter().map(|t| log.with_traces(t)).collect()
    }
}
