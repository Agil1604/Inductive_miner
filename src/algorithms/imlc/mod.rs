use crate::InductiveMiner;

use crate::components::base_cases::{
    BaseCaseChain, EmptyLog, SingleNonAtomicActivity, WithoutEmptyTraces,
};
use crate::components::cut_detections::{
    CutChain, NonAtomicConcurrentCut, NonAtomicCutFinder, NonAtomicDfgCut, NonAtomicInterleavedCut,
    NonAtomicLoopCut, SequenceCut, XorCut,
};
use crate::components::fall_throughs::{
    ActivityConcurrent, ConcurrentFlowerModel, EmptyTraces, FallThroughChain, FallThroughFinder,
    NonAtomicActivityOncePerTrace, NonAtomicTauLoop, StrictNonAtomicTauLoop,
};
use crate::components::log_splitting::{
    ConcurrentSplit, ExtendedLogSplitter, InclusiveChoiceSplit, InterleavedSplit, LogSplitter,
    LoopSplit, SequenceSplit, XorSplit,
};

pub type IMlc = InductiveMiner<ImlcBaseCase, ImlcCutFinder, ImlcLogSplitter, ImlcFallThrough>;

pub type ImlcBaseCase = WithoutEmptyTraces<BaseCaseChain<EmptyLog, SingleNonAtomicActivity>>;

pub type ImlcCutFinder = NonAtomicCutFinder<
    CutChain<
        NonAtomicDfgCut<XorCut>,
        CutChain<
            NonAtomicDfgCut<SequenceCut>,
            CutChain<NonAtomicConcurrentCut, CutChain<NonAtomicInterleavedCut, NonAtomicLoopCut>>,
        >,
    >,
>;

pub type ImlcLogSplitter = ExtendedLogSplitter<
    LogSplitter<XorSplit, SequenceSplit, ConcurrentSplit, LoopSplit>,
    InterleavedSplit,
    InclusiveChoiceSplit,
>;

pub type ImlcFallThrough = FallThroughFinder<
    FallThroughChain<
        EmptyTraces,
        FallThroughChain<
            NonAtomicActivityOncePerTrace,
            FallThroughChain<
                ActivityConcurrent,
                FallThroughChain<StrictNonAtomicTauLoop, NonAtomicTauLoop>,
            >,
        >,
    >,
    ConcurrentFlowerModel,
>;

#[cfg(test)]
mod tests;
