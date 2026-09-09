use super::project;
use crate::framework::SplitLog;
use crate::{IndexedCut, IndexedEventLog, OperatorType};

/// Splits a log by projecting traces onto inclusive-choice cut partitions.
///
/// Returns one sublog per partition, in partition order, discarding empty
/// projections. Only cases that execute a partition contribute to its sublog.
/// Retained events keep their order, lifecycle, and timestamps; case IDs and
/// duplicate nonempty projections are preserved. All sublogs share the input
/// log's activity interner.
///
/// A cut with an operator other than [`OperatorType::InclusiveChoice`] returns
/// an empty vector.
#[derive(Debug, Default, Clone, Copy)]
pub struct InclusiveChoiceSplit;
impl SplitLog for InclusiveChoiceSplit {
    fn split_log(&self, log: &IndexedEventLog, cut: &IndexedCut) -> Vec<IndexedEventLog> {
        if cut.operator != OperatorType::InclusiveChoice {
            return Vec::new();
        }
        cut.partitions
            .iter()
            .map(|p| {
                let projected = project(log, |a| p.contains(a));
                log.with_traces(
                    projected
                        .traces
                        .iter()
                        .filter(|t| !t.events.is_empty())
                        .cloned()
                        .collect(),
                )
            })
            .collect()
    }
}
