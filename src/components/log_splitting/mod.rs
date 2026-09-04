//! Reusable operator-specific log splitters for standard Inductive Miner.
mod common;
mod concurrent_split;
mod loop_split;
mod sequence_split;
mod xor_split;

pub(crate) use common::project;
pub use concurrent_split::ConcurrentSplit;
pub use loop_split::LoopSplit;
pub use sequence_split::SequenceSplit;
pub use xor_split::XorSplit;

use crate::framework::SplitLog;
use crate::{IndexedCut, IndexedEventLog, OperatorType};

///
/// Dispatches to the component for the cut's operator.
///
#[derive(Debug, Clone, Copy, Default)]
pub struct LogSplitter<X, S, C, L> {
    pub xor: X,
    pub sequence: S,
    pub concurrent: C,
    pub loop_split: L,
}
impl<X: SplitLog, S: SplitLog, C: SplitLog, L: SplitLog> SplitLog for LogSplitter<X, S, C, L> {
    fn split_log(&self, log: &IndexedEventLog, cut: &IndexedCut) -> Vec<IndexedEventLog> {
        match cut.operator {
            OperatorType::Xor => self.xor.split_log(log, cut),
            OperatorType::Sequence => self.sequence.split_log(log, cut),
            OperatorType::Concurrent => self.concurrent.split_log(log, cut),
            OperatorType::Loop => self.loop_split.split_log(log, cut),
            OperatorType::Interleaved => Vec::new(),
            OperatorType::InclusiveChoice => Vec::new(), 
        }
    }
}

#[cfg(test)]
mod tests;
