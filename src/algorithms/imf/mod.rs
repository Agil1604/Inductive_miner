use crate::InductiveMiner;

use crate::components::base_cases::{
    BaseCaseChain, EmptyLog, SingleActivityFiltering, WithoutEmptyTraces,
};
use crate::components::cut_detections::{
    ConcurrentCut, CutChain, FilteringCutFinder, LoopCut, SequenceCut, XorCut,
};
use crate::components::fall_throughs::{
    ActivityConcurrent, ActivityOncePerTrace, EmptyTracesFiltering, FallThroughChain,
    FallThroughFinder, FlowerModel, StrictTauLoop, TauLoop,
};
use crate::components::filtering::FilteringConfig;
use crate::components::log_splitting::{
    ConcurrentSplit, LogSplitter, LoopSplitFiltering, SequenceSplitFiltering, XorSplitFiltering,
};

/// Inductive Miner with infrequent/deviating behaviour filtering.
pub type IMf = InductiveMiner<ImfBaseCase, ImfCutFinder, ImfLogSplitter, ImfFallThrough>;

pub type ImfBaseCase = WithoutEmptyTraces<BaseCaseChain<EmptyLog, SingleActivityFiltering>>;

pub type ImfCutFinder =
    FilteringCutFinder<CutChain<XorCut, CutChain<SequenceCut, CutChain<ConcurrentCut, LoopCut>>>>;

pub type ImfLogSplitter =
    LogSplitter<XorSplitFiltering, SequenceSplitFiltering, ConcurrentSplit, LoopSplitFiltering>;

pub type ImfFallThrough = FallThroughFinder<
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
    /// Builds the filtering base cases shared by IMf and IMfa.
    pub fn base_cases(self) -> ImfBaseCase {
        WithoutEmptyTraces(BaseCaseChain {
            first: EmptyLog,
            second: SingleActivityFiltering { config: self },
        })
    }

    /// Builds IMf's unfiltered-first cut finder with this threshold.
    pub fn imf_cut_finder(self) -> ImfCutFinder {
        FilteringCutFinder {
            strategy: Default::default(),
            config: self,
        }
    }

    /// Builds IMf splitting for the four basic operators.
    pub fn imf_log_splitter(self) -> ImfLogSplitter {
        ImfLogSplitter::default()
    }

    /// Builds the filtering fall-throughs shared by IMf and IMfa.
    pub fn fall_through(self) -> ImfFallThrough {
        FallThroughFinder {
            strategies: FallThroughChain {
                first: EmptyTracesFiltering { config: self },
                second: Default::default(),
            },
            fallback: FlowerModel,
        }
    }

    /// Builds a complete miner with the same configuration at every recursion.
    pub fn imf(self) -> IMf {
        InductiveMiner::new(
            self.base_cases(),
            self.imf_cut_finder(),
            self.imf_log_splitter(),
            self.fall_through(),
        )
    }
}

#[cfg(test)]
mod tests;
