mod common;
mod concurrent_cut;
mod coo_cut;
mod filtering;
mod interleaved_cut;
mod loop_cut;
mod non_atomic;
mod probabilistic;
mod sequence_cut;
mod sequence_cut_strict;
mod xor_cut;

pub use concurrent_cut::ConcurrentCut;
pub use coo_cut::CooCut;
pub use filtering::{FilteringCutFinder, TwoPassFilteringCutFinder, filter_dfg};
pub use interleaved_cut::{InterleavedCut, InterleavedCutFiltering};
pub use loop_cut::LoopCut;
pub use non_atomic::{
    NonAtomicConcurrentCut, NonAtomicCutContext, NonAtomicCutFinder, NonAtomicCutStrategy,
    NonAtomicDfgCut, NonAtomicInterleavedCut, NonAtomicLoopCut,
};
pub use probabilistic::{ImcCutFinder, ImclcCutFinder};
pub use sequence_cut::SequenceCut;
pub use sequence_cut_strict::SequenceCutStrict;
pub use xor_cut::XorCut;

use crate::framework::DetectCut;
use crate::{ActivityId, IndexedCut, IndexedDfg, IndexedEventLog};

use std::cell::OnceCell;
use std::collections::HashSet;

/// Prepared input shared by all strategies in one cut-detection call.
///
/// Borrows the indexed log and owns its sorted alphabet and DFG. Minimum
/// self-distance witnesses are computed lazily from the original log and cached.
/// A [`CutChain`] reuses this context rather than rebuilding the DFG per strategy.
/// Construction requires at least two distinct activities and no empty traces.
#[derive(Debug)]
pub struct CutContext<'a> {
    log: &'a IndexedEventLog,
    activities: Vec<ActivityId>,
    dfg: IndexedDfg,
    witnesses: OnceCell<HashSet<(ActivityId, ActivityId)>>,
}

impl<'a> CutContext<'a> {
    /// Borrows `log`, collects its sorted activity IDs, and builds its DFG once.
    ///
    /// Returns `None` if any trace is empty or fewer than two distinct activities
    /// occur, including a log with no traces. Witness computation is deferred
    /// until [`Self::witnesses`] is called. Events are used in stored order;
    /// timestamps and lifecycle values do not affect preparation.
    pub fn new(log: &'a IndexedEventLog) -> Option<Self> {
        let (activities, dfg) = common::context(log)?;
        Some(Self {
            log,
            activities,
            dfg,
            witnesses: OnceCell::new(),
        })
    }

    /// Creates a new context using `dfg` while retaining the original log evidence.
    ///
    /// Borrows the same log and clones the alphabet. If witnesses have already
    /// been computed, their cached set is cloned; otherwise the new context
    /// computes them independently on first use. They always come from the
    /// original log, not the replacement graph.
    ///
    /// Used for filtered cut detection. The caller must preserve the activity
    /// alphabet and the originating interner's IDs in `dfg`; this method does
    /// not validate the replacement graph.
    pub fn with_dfg(&self, dfg: IndexedDfg) -> Self {
        Self {
            log: self.log,
            activities: self.activities.clone(),
            dfg,
            witnesses: self.witnesses.clone(),
        }
    }

    /// Immutable getter for the log.
    pub fn log(&self) -> &IndexedEventLog {
        self.log
    }

    /// Immutable getter for the activity alphabet.
    pub fn activities(&self) -> &[ActivityId] {
        &self.activities
    }

    /// Immutable getter for the DFG abstraction.
    pub fn dfg(&self) -> &IndexedDfg {
        &self.dfg
    }

    /// Returns cached minimum self-distance witnesses from the original log.
    ///
    /// A pair `(a, b)` means that `b` occurs between two consecutive occurrences
    /// of `a` separated by the minimum number of intervening events observed
    /// for `a` across the log. All pairs tied at that minimum contribute their
    /// intervening activities. Activities that never repeat contribute nothing;
    /// an adjacent repetition has distance zero and contributes no witnesses.
    ///
    /// Computed on first access and reused afterward. The relation is directed:
    /// `(a, b)` does not imply `(b, a)`.
    pub fn witnesses(&self) -> &HashSet<(ActivityId, ActivityId)> {
        self.witnesses.get_or_init(|| common::witnesses(self.log))
    }
}

/// A reusable cut detector operating on an already prepared [`CutContext`].
///
/// Implementations can share the DFG and lazy witnesses when composed with
/// [`CutChain`]. Use [`CutFinder`] to adapt a strategy to the framework's
/// [`DetectCut`] trait, which accepts an indexed log directly.
pub trait CutStrategy {
    /// Returns a cut when this strategy applies, or `None` to allow the next
    /// strategy to run.
    ///
    /// A returned cut must have at least two nonempty, disjoint partitions
    /// covering [`CutContext::activities`], using IDs from the context's log.
    /// Partition order defines sequence order and the body/redo roles of loops.
    /// Chains and adapters do not validate the result; the recursive miner does.
    fn detect(&self, context: &CutContext<'_>) -> Option<IndexedCut>;
}

/// First-match composition of two [`CutStrategy`] implementations.
///
/// Both receive the same context. The second runs only if the first returns
/// `None`; results are not scored or compared. Nest chains to express a longer
/// priority order without rebuilding the log abstractions.
#[derive(Debug, Clone, Copy, Default)]
pub struct CutChain<A, B> {
    /// Strategy attempted first.
    pub first: A,
    /// Strategy attempted only when the first returns no cut.
    pub second: B,
}
impl<A: CutStrategy, B: CutStrategy> CutStrategy for CutChain<A, B> {
    fn detect(&self, context: &CutContext<'_>) -> Option<IndexedCut> {
        self.first
            .detect(context)
            .or_else(|| self.second.detect(context))
    }
}

/// Adapts a [`CutStrategy`] or [`CutChain`] to the framework's [`DetectCut`] trait.
///
/// Each call creates one [`CutContext`] and passes it to the wrapped strategy.
/// Unsuitable logs are rejected before invoking the strategy. Returned cuts are
/// passed through without validation.
#[derive(Debug, Clone, Copy, Default)]
pub struct CutFinder<S>(
    /// Strategy or ordered chain used to find a cut.
    pub S,
);

impl<S: CutStrategy> DetectCut for CutFinder<S> {
    fn detect_cut(&self, log: &IndexedEventLog) -> Option<IndexedCut> {
        let context = CutContext::new(log)?;
        self.0.detect(&context)
    }
}

#[cfg(test)]
mod tests;

pub use non_atomic::{NonAtomicFilteringCutFinder, NonAtomicInterleavedCutFiltering};
