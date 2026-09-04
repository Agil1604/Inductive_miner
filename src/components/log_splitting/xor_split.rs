use crate::framework::SplitLog;
use crate::{IndexedCut, IndexedEventLog, OperatorType};

///
/// Assigns each complete trace to its exclusive-choice branch.
/// Returns one sublog per partition in partition order.
///
#[derive(Debug, Clone, Copy, Default)]
pub struct XorSplit;
impl SplitLog for XorSplit {
    fn split_log(&self, log: &IndexedEventLog, cut: &IndexedCut) -> Vec<IndexedEventLog> {
        if cut.operator != OperatorType::Xor {
            return Vec::new();
        }
        cut.partitions
            .iter()
            .map(|part| {
                log.with_traces(
                    log.traces
                        .iter()
                        .filter(|t| t.events.iter().all(|e| part.contains(&e.activity)))
                        .cloned()
                        .collect(),
                )
            })
            .collect()
    }
}
