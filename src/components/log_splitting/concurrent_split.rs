use super::common::project;
use crate::framework::SplitLog;
use crate::{IndexedCut, IndexedEventLog, OperatorType};

///
/// Projects every trace onto every parallel branch, retaining empty projections.
/// Returns one sublog per partition in partition order.
///
#[derive(Debug, Clone, Copy, Default)]
pub struct ConcurrentSplit;
impl SplitLog for ConcurrentSplit {
    fn split_log(&self, log: &IndexedEventLog, cut: &IndexedCut) -> Vec<IndexedEventLog> {
        if cut.operator != OperatorType::Concurrent {
            return Vec::new();
        }
        cut.partitions
            .iter()
            .map(|part| project(log, |a| part.contains(a)))
            .collect()
    }
}
