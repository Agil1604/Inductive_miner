use crate::InductiveMiner;

use crate::components::base_cases::{BaseCaseChain, EmptyLog, SingleActivity};
use crate::components::cut_detections::{
    CooCut, CutChain, CutFinder, InterleavedCut, LoopCut, SequenceCutStrict, XorCut,
};
use crate::components::fall_throughs::{
    ActivityConcurrent, ActivityOncePerTrace, EmptyTraces, FallThroughChain, FallThroughFinder,
    FlowerModel, StrictTauLoop, TauLoop,
};
use crate::components::log_splitting::{
    ConcurrentSplit, ExtendedLogSplitter, InclusiveChoiceSplit, InterleavedSplit, LogSplitter,
    LoopSplit, SequenceSplit, XorSplit,
};

/// Inductive Miner with all six operators, including interleaved and inclusive choice.
///
/// Construct with `IMa::default()` and import [`crate::Miner`] to call `mine`.
pub type IMa = InductiveMiner<ImaBaseCase, ImaCutFinder, ImaLogSplitter, ImaFallThrough>;

pub type ImaBaseCase = BaseCaseChain<EmptyLog, SingleActivity>;

pub type ImaCutFinder = CutFinder<
    CutChain<
        XorCut,
        CutChain<SequenceCutStrict, CutChain<InterleavedCut, CutChain<CooCut, LoopCut>>>,
    >,
>;

pub type ImaLogSplitter = ExtendedLogSplitter<
    LogSplitter<XorSplit, SequenceSplit, ConcurrentSplit, LoopSplit>,
    InterleavedSplit,
    InclusiveChoiceSplit,
>;

pub type ImaFallThrough = FallThroughFinder<
    FallThroughChain<
        EmptyTraces,
        FallThroughChain<
            ActivityOncePerTrace,
            FallThroughChain<ActivityConcurrent, FallThroughChain<StrictTauLoop, TauLoop>>,
        >,
    >,
    FlowerModel,
>;

#[cfg(test)]
mod tests;
