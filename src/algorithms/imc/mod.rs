use crate::InductiveMiner;

use crate::components::base_cases::{BaseCaseChain, EmptyLog, SingleActivity};
pub use crate::components::cut_detections::ImcCutFinder;
use crate::components::fall_throughs::{EmptyTraces, FallThroughFinder, FlowerModel};
use crate::components::log_splitting::{
    ConcurrentSplit, LogSplitter, LoopSplitFiltering, SequenceSplitFiltering, XorSplitFiltering,
};

/// Inductive Miner for incomplete behaviour, using probabilistic
/// SMT cut detection when standard IM finds no cut.
///
/// Construct with `IMc::default()` and import [`crate::Miner`] to call `mine`.
pub type IMc = InductiveMiner<ImcBaseCase, ImcCutFinder, ImcLogSplitter, ImcFallThrough>;

pub type ImcBaseCase = BaseCaseChain<EmptyLog, SingleActivity>;

pub type ImcLogSplitter =
    LogSplitter<XorSplitFiltering, SequenceSplitFiltering, ConcurrentSplit, LoopSplitFiltering>;

pub type ImcFallThrough = FallThroughFinder<EmptyTraces, FlowerModel>;


#[cfg(test)]
mod tests;
