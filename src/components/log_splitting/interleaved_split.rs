use super::project;
use crate::framework::SplitLog;
use crate::{IndexedCut, IndexedEventLog, OperatorType};

/// Splits a log by projecting every trace onto each interleaved cut partition.
///
/// Returns one sublog per partition, in partition order. Each sublog contains
/// one trace per input trace, including empty projections. Retained events keep
/// their order, lifecycle, and timestamps; case IDs and duplicate traces are
/// preserved. All sublogs share the input log's activity interner.
///
/// A cut with an operator other than [`OperatorType::Interleaved`] returns an
/// empty vector.
#[derive(Debug, Default, Clone, Copy)]
pub struct InterleavedSplit;
impl SplitLog for InterleavedSplit {
    fn split_log(&self, log: &IndexedEventLog, cut: &IndexedCut) -> Vec<IndexedEventLog> {
        if cut.operator != OperatorType::Interleaved {
            return Vec::new();
        }
        cut.partitions
            .iter()
            .map(|p| project(log, |a| p.contains(a)))
            .collect()
    }
}
