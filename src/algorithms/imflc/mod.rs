use crate::InductiveMiner;

use crate::components::base_cases::{
    BaseCaseChain, EmptyLog, SingleNonAtomicActivityFiltering, WithoutEmptyTraces,
};
use crate::components::cut_detections::{
    CutChain, NonAtomicConcurrentCut, NonAtomicCutFinder, NonAtomicDfgCut,
    NonAtomicFilteringCutFinder, NonAtomicInterleavedCut, NonAtomicInterleavedCutFiltering,
    NonAtomicLoopCut, SequenceCut, XorCut,
};
use crate::components::fall_throughs::{
    ActivityConcurrent, ConcurrentFlowerModel, EmptyTraces, EmptyTracesFiltering, FallThroughChain,
    FallThroughFinder, NonAtomicActivityOncePerTrace, NonAtomicTauLoop, StrictNonAtomicTauLoop,
};
use crate::components::log_splitting::{
    ConcurrentSplit, ConsistentSplit, ExtendedLogSplitter, InclusiveChoiceSplit,
    InterleavedSplitFiltering, LogSplitter, LoopSplitFiltering, SequenceSplitFiltering,
    XorSplitFiltering,
};

use crate::components::filtering::FilteringConfig;

/// Lifecycle-aware Inductive Miner for infrequent and deviating behaviour.
/// Construct with `IMflc::default()` or [`FilteringConfig::imflc`].
/// Filtering may discard behaviour; split outputs are repaired for consistency.
pub type IMflc = InductiveMiner<ImflcBaseCase, ImflcCutFinder, ImflcLogSplitter, ImflcFallThrough>;

pub type ImflcBaseCase =
    WithoutEmptyTraces<BaseCaseChain<EmptyLog, SingleNonAtomicActivityFiltering>>;

pub type ImflcFilteredCuts = CutChain<
    NonAtomicDfgCut<XorCut>,
    CutChain<
        NonAtomicDfgCut<SequenceCut>,
        CutChain<
            NonAtomicConcurrentCut,
            CutChain<NonAtomicInterleavedCutFiltering, NonAtomicLoopCut>,
        >,
    >,
>;

pub type ImflcCutFinder = NonAtomicFilteringCutFinder<
    NonAtomicCutFinder<
        CutChain<
            NonAtomicDfgCut<XorCut>,
            CutChain<
                NonAtomicDfgCut<SequenceCut>,
                CutChain<
                    NonAtomicConcurrentCut,
                    CutChain<NonAtomicInterleavedCut, NonAtomicLoopCut>,
                >,
            >,
        >,
    >,
    ImflcFilteredCuts,
>;

pub type ImflcLogSplitter = ExtendedLogSplitter<
    LogSplitter<
        XorSplitFiltering,
        ConsistentSplit<SequenceSplitFiltering>,
        ConcurrentSplit,
        ConsistentSplit<LoopSplitFiltering>,
    >,
    ConsistentSplit<InterleavedSplitFiltering>,
    InclusiveChoiceSplit,
>;

pub type ImflcFallThrough = FallThroughFinder<
    EmptyTracesFiltering,
    FallThroughFinder<
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
    >,
>;

impl FilteringConfig {
    /// Builds IMflc with a shared threshold across its filtering components.
    pub fn imflc(self) -> IMflc {
        InductiveMiner::new(
            WithoutEmptyTraces(BaseCaseChain {
                first: EmptyLog,
                second: SingleNonAtomicActivityFiltering { config: self },
            }),
            NonAtomicFilteringCutFinder {
                unfiltered: Default::default(),
                filtered: Default::default(),
                config: self,
            },
            Default::default(),
            FallThroughFinder {
                strategies: EmptyTracesFiltering { config: self },
                fallback: Default::default(),
            },
        )
    }
}
#[cfg(test)]
mod tests;
