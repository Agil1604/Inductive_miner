//! Reusable operator-specific log splitters for standard and infrequent Inductive Miner.
mod common;
mod concurrent_split;
mod inclusive_choice_split;
mod interleaved_split;
mod loop_split;
mod loop_split_filtering;
mod sequence_split;
mod sequence_split_filtering;
mod xor_split;
mod xor_split_filtering;

pub(crate) use common::project;
pub use concurrent_split::ConcurrentSplit;
pub use inclusive_choice_split::InclusiveChoiceSplit;
pub use interleaved_split::InterleavedSplit;
pub use loop_split::LoopSplit;
pub use loop_split_filtering::LoopSplitFiltering;
pub use sequence_split::SequenceSplit;
pub use sequence_split_filtering::SequenceSplitFiltering;
pub use xor_split::XorSplit;
pub use xor_split_filtering::XorSplitFiltering;

use crate::framework::SplitLog;
use crate::{IndexedCut, IndexedEventLog, OperatorType};

/// Dispatches to the component for the cut's operator.
#[derive(Debug, Clone, Copy, Default)]
pub struct LogSplitter<X, S, C, L> {
    /// Splits logs for [`OperatorType::Xor`] cuts.
    pub xor: X,
    /// Splits logs for [`OperatorType::Sequence`] cuts in partition order.
    pub sequence: S,
    /// Splits logs for [`OperatorType::Concurrent`] cuts.
    pub concurrent: C,
    /// Splits logs for [`OperatorType::Loop`] cuts, with the body first and
    /// alternative redo parts afterward.
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

/// Extends a basic operator dispatcher with interleaved and inclusive choice.
#[derive(Debug, Default, Clone, Copy)]
pub struct ExtendedLogSplitter<S, I, O> {
    /// Splits logs for [`OperatorType::Xor`], [`OperatorType::Sequence`],
    /// [`OperatorType::Concurrent`], and [`OperatorType::Loop`] cuts.
    pub standard: S,
    /// Splits logs for [`OperatorType::Interleaved`] cuts.
    pub interleaved: I,
    /// Splits logs for [`OperatorType::InclusiveChoice`] cuts.
    pub inclusive_choice: O,
}
impl<S: SplitLog, I: SplitLog, O: SplitLog> SplitLog for ExtendedLogSplitter<S, I, O> {
    fn split_log(&self, log: &IndexedEventLog, cut: &IndexedCut) -> Vec<IndexedEventLog> {
        match cut.operator {
            OperatorType::Interleaved => self.interleaved.split_log(log, cut),
            OperatorType::InclusiveChoice => self.inclusive_choice.split_log(log, cut),
            _ => self.standard.split_log(log, cut),
        }
    }
}
