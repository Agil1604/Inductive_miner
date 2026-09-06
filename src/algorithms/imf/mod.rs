mod config;
pub use config::{ImfConfig, InvalidDeviationThreshold};

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
use crate::components::log_splitting::{
    ConcurrentSplit, LogSplitter, LoopSplitFiltering, SequenceSplitFiltering, XorSplitFiltering,
};

///
/// Inductive Miner with infrequent/deviating behaviour filtering.
/// 
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


impl ImfConfig {
    pub fn base_cases(self) -> ImfBaseCase {
        WithoutEmptyTraces(BaseCaseChain {
            first: EmptyLog,
            second: SingleActivityFiltering { config: self },
        })
    }

    pub fn cut_finder(self) -> ImfCutFinder {
        FilteringCutFinder {
            strategy: Default::default(),
            config: self,
        }
    }

    pub fn log_splitter(self) -> ImfLogSplitter {
        ImfLogSplitter::default()
    }

    pub fn fall_through(self) -> ImfFallThrough {
        FallThroughFinder {
            strategies: FallThroughChain {
                first: EmptyTracesFiltering { config: self },
                second: Default::default(),
            },
            fallback: FlowerModel,
        }
    }

    ///
    /// Builds a complete miner with the same configuration at every recursion.
    ///
    pub fn miner(self) -> IMf {
        InductiveMiner::new(
            self.base_cases(),
            self.cut_finder(),
            self.log_splitter(),
            self.fall_through(),
        )
    }
}

#[cfg(test)]
mod tests;
