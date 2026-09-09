mod common;
mod concurrent_cut;
mod filtering;
mod probabilistic;
mod loop_cut;
mod sequence_cut;
mod xor_cut;

pub use concurrent_cut::ConcurrentCut;
pub use filtering::{FilteringCutFinder, filter_dfg};
pub use probabilistic::ImcCutFinder;
pub use loop_cut::LoopCut;
pub use sequence_cut::SequenceCut;
pub use xor_cut::XorCut;

use crate::framework::DetectCut;
use crate::{ActivityId, IndexedCut, IndexedDfg, IndexedEventLog};

use std::cell::OnceCell;
use std::collections::HashSet;

///
/// Prepared input shared by all strategies in one cut-detection call.
/// Construction rejects logs with empty traces or fewer than two activities.
///
#[derive(Debug)]
pub struct CutContext<'a> {
    log: &'a IndexedEventLog,
    activities: Vec<ActivityId>,
    dfg: IndexedDfg,
    witnesses: OnceCell<HashSet<(ActivityId, ActivityId)>>,
}

impl<'a> CutContext<'a> {
    ///
    /// Prepares a log for cut detection, or returns `None` if unsuitable.
    ///
    pub fn new(log: &'a IndexedEventLog) -> Option<Self> {
        let (activities, dfg) = common::context(log)?;
        Some(Self {
            log,
            activities,
            dfg,
            witnesses: OnceCell::new(),
        })
    }

    ///
    /// Preserve the original alphabet and cached witnesses when changing only
    /// the DFG abstraction for a filtering pass.
    ///
    pub fn with_dfg(&self, dfg: IndexedDfg) -> Self {
        Self {
            log: self.log,
            activities: self.activities.clone(),
            dfg,
            witnesses: self.witnesses.clone(),
        }
    }

    pub fn log(&self) -> &IndexedEventLog {
        self.log
    }

    pub fn activities(&self) -> &[ActivityId] {
        &self.activities
    }

    pub fn dfg(&self) -> &IndexedDfg {
        &self.dfg
    }

    ///
    /// Minimum self-distance witnesses, computed on first use.
    ///
    pub fn witnesses(&self) -> &HashSet<(ActivityId, ActivityId)> {
        self.witnesses.get_or_init(|| common::witnesses(self.log))
    }
}

///
/// A cut-detection strategy consuming already prepared input.
///
pub trait CutStrategy {
    ///
    /// Returns a nontrivial cut, or `None` when this strategy does not apply.
    ///
    fn detect(&self, context: &CutContext<'_>) -> Option<IndexedCut>;
}

///
/// Tries the first strategy, then the second only if no cut was found.
///
#[derive(Debug, Clone, Copy, Default)]
pub struct CutChain<A, B> {
    pub first: A,
    pub second: B,
}
impl<A: CutStrategy, B: CutStrategy> CutStrategy for CutChain<A, B> {
    fn detect(&self, context: &CutContext<'_>) -> Option<IndexedCut> {
        self.first
            .detect(context)
            .or_else(|| self.second.detect(context))
    }
}

///
/// Adapts a strategy or chain to the framework, building one context per call.
///
#[derive(Debug, Clone, Copy, Default)]
pub struct CutFinder<S>(pub S);

impl<S: CutStrategy> DetectCut for CutFinder<S> {
    fn detect_cut(&self, log: &IndexedEventLog) -> Option<IndexedCut> {
        let context = CutContext::new(log)?;
        self.0.detect(&context)
    }
}

#[cfg(test)]
mod tests;
