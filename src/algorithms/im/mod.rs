use crate::InductiveMiner;
use crate::components::base_cases::{BaseCaseChain, EmptyLog, SingleActivity};
use crate::components::cut_detections::{
    ConcurrentCut, CutChain, CutFinder, LoopCut, SequenceCut, XorCut,
};
use crate::components::fall_throughs::{
    ActivityConcurrent, ActivityOncePerTrace, EmptyTraces, FallThroughChain, FallThroughFinder,
    FlowerModel, StrictTauLoop, TauLoop,
};
use crate::components::log_splitting::{
    ConcurrentSplit, LogSplitter, LoopSplit, SequenceSplit, XorSplit,
};

///
/// Standard IM. Construct with `IM::default()` and import [`crate::Miner`] to call `mine`.
///
pub type IM = InductiveMiner<ImBaseCase, ImCutFinder, ImLogSplitter, ImFallThrough>;

pub type ImBaseCase = BaseCaseChain<EmptyLog, SingleActivity>;

pub type ImCutFinder =
    CutFinder<CutChain<XorCut, CutChain<SequenceCut, CutChain<ConcurrentCut, LoopCut>>>>;

pub type ImLogSplitter = LogSplitter<XorSplit, SequenceSplit, ConcurrentSplit, LoopSplit>;

pub type ImFallThrough = FallThroughFinder<
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
