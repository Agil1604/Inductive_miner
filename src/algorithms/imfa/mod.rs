use crate::InductiveMiner;

use crate::components::base_cases::{
    BaseCaseChain, EmptyLog, SingleActivityFiltering, WithoutEmptyTraces,
};
use crate::components::cut_detections::{
    CooCut, CutChain, InterleavedCut, InterleavedCutFiltering, LoopCut, SequenceCutStrict,
    TwoPassFilteringCutFinder, XorCut,
};
use crate::components::fall_throughs::{
    ActivityConcurrent, ActivityOncePerTrace, EmptyTracesFiltering, FallThroughChain,
    FallThroughFinder, FlowerModel, StrictTauLoop, TauLoop,
};
use crate::components::log_splitting::{
    ConcurrentSplit, ExtendedLogSplitter, InclusiveChoiceSplit, InterleavedSplitFiltering,
    LogSplitter, LoopSplitFiltering, SequenceSplitFiltering, XorSplitFiltering,
};

use crate::components::filtering::FilteringConfig;

/// Complete IMfa miner. Filtering can discard observed behaviour.
///
/// Construct with `IMfa::default()` or [`FilteringConfig::imfa`].
pub type IMfa = InductiveMiner<ImfaBaseCase, ImfaCutFinder, ImfaLogSplitter, ImfaFallThrough>;

pub type ImfaBaseCase = WithoutEmptyTraces<BaseCaseChain<EmptyLog, SingleActivityFiltering>>;

pub type ImfaUnfilteredCuts = CutChain<
    XorCut,
    CutChain<SequenceCutStrict, CutChain<InterleavedCut, CutChain<CooCut, LoopCut>>>,
>;

pub type ImfaFilteredCuts =
    CutChain<XorCut, CutChain<SequenceCutStrict, CutChain<InterleavedCutFiltering, LoopCut>>>;

pub type ImfaCutFinder = TwoPassFilteringCutFinder<ImfaUnfilteredCuts, ImfaFilteredCuts>;

pub type ImfaLogSplitter = ExtendedLogSplitter<
    LogSplitter<XorSplitFiltering, SequenceSplitFiltering, ConcurrentSplit, LoopSplitFiltering>,
    InterleavedSplitFiltering,
    InclusiveChoiceSplit,
>;

pub type ImfaFallThrough = FallThroughFinder<
    FallThroughChain<
        EmptyTracesFiltering,
        FallThroughChain<
            ActivityOncePerTrace,
            FallThroughChain<ActivityConcurrent, FallThroughChain<StrictTauLoop, TauLoop>>,
        >,
    >,
    FlowerModel,
>;

impl FilteringConfig {
    /// Builds IMa-first detection and the filtered fallback chain.
    pub fn imfa_cut_finder(self) -> ImfaCutFinder {
        TwoPassFilteringCutFinder {
            unfiltered: Default::default(),
            filtered: Default::default(),
            config: self,
        }
    }
    /// Builds splitters for all six supported operators.
    pub fn imfa_log_splitter(self) -> ImfaLogSplitter {
        Default::default()
    }
    /// Builds a complete miner with consistent configuration throughout recursion.
    ///
    /// ```
    /// use inductive_miner::{EventLog, FilteringConfig, Miner};
    /// let tree = FilteringConfig::new(0.15)?.imfa().mine(&EventLog::default())?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn imfa(self) -> IMfa {
        InductiveMiner::new(
            self.base_cases(),
            self.imfa_cut_finder(),
            self.imfa_log_splitter(),
            self.fall_through(),
        )
    }
}

#[cfg(test)]
mod tests;
