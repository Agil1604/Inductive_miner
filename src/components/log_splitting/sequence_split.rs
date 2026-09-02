use super::common::project;
use crate::framework::SplitLog;
use crate::{IndexedCut, IndexedEventLog, OperatorType};

///
/// Projects traces onto the ordered sequence parts, retaining empty projections.
/// Returns one sublog per partition in partition order.
///
#[derive(Debug, Clone, Copy, Default)]
pub struct SequenceSplit;
impl SplitLog for SequenceSplit {
    fn split_log(&self, log: &IndexedEventLog, cut: &IndexedCut) -> Vec<IndexedEventLog> {
        if cut.operator != OperatorType::Sequence {
            return Vec::new();
        }
        cut.partitions
            .iter()
            .map(|part| project(log, |a| part.contains(a)))
            .collect()
    }
}
