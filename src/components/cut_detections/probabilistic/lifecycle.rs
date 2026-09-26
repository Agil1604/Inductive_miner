use super::{ActivityRelations, ScoredCut, smt};
use crate::algorithms::imlc::ImlcCutFinder;
use crate::components::cut_detections::NonAtomicCutContext;
use crate::framework::DetectCut;
use crate::{IndexedCut, IndexedEventLog};

/// IMclc cut detection: IMlc first, then lifecycle-aware SMT optimization.
/// Observed overlaps are concurrent relations; the existing IMc objective and
/// deterministic tie-breaking are reused.
#[derive(Debug, Clone, Copy, Default)]
pub struct ImclcCutFinder;
impl ImclcCutFinder {
    /// Optimizes binary cuts using lifecycle evidence, without the IMlc first pass.
    /// Returns `None` for effective empty traces or fewer than two activities.
    ///
    /// # Panics
    /// Panics if Z3 reports an unknown result or cannot provide a model.
    pub fn find_scored_cut(&self, log: &IndexedEventLog) -> Option<ScoredCut> {
        NonAtomicCutContext::new(log)?;
        smt::find_best(&ActivityRelations::from_non_atomic_log(log))
    }
}
impl DetectCut for ImclcCutFinder {
    fn detect_cut(&self, log: &IndexedEventLog) -> Option<IndexedCut> {
        ImlcCutFinder::default()
            .detect_cut(log)
            .or_else(|| self.find_scored_cut(log).map(|r| r.cut))
    }
}
