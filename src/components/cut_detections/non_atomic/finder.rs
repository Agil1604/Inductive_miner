use super::super::{CutChain, CutStrategy};
use super::{NonAtomicCutContext, NonAtomicCutStrategy};
use crate::framework::DetectCut;
use crate::{IndexedCut, IndexedEventLog};

/// Adapts an atomic DFG strategy to the lifecycle DFG with overlap connections.
/// Symmetric overlap edges prevent XOR and sequence cuts from separating
/// overlapping executions. The prepared graph is shared across adapters.
#[derive(Debug, Clone, Copy, Default)]
pub struct NonAtomicDfgCut<S>(pub S);
impl<S: CutStrategy> NonAtomicCutStrategy for NonAtomicDfgCut<S> {
    fn detect(&self, context: &NonAtomicCutContext<'_>) -> Option<IndexedCut> {
        self.0.detect(context.dfg_context())
    }
}

impl<A: NonAtomicCutStrategy, B: NonAtomicCutStrategy> NonAtomicCutStrategy for CutChain<A, B> {
    fn detect(&self, context: &NonAtomicCutContext<'_>) -> Option<IndexedCut> {
        self.first
            .detect(context)
            .or_else(|| self.second.detect(context))
    }
}

/// Adapts a lifecycle strategy or [`CutChain`] to [`DetectCut`].
/// Builds one shared lifecycle context per call and returns the first cut found.
#[derive(Debug, Clone, Copy, Default)]
pub struct NonAtomicCutFinder<S>(pub S);
impl<S: NonAtomicCutStrategy> DetectCut for NonAtomicCutFinder<S> {
    fn detect_cut(&self, log: &IndexedEventLog) -> Option<IndexedCut> {
        self.0.detect(&NonAtomicCutContext::new(log)?)
    }
}

impl<S: NonAtomicCutStrategy> NonAtomicCutStrategy for NonAtomicCutFinder<S> {
    fn detect(&self, context: &NonAtomicCutContext<'_>) -> Option<IndexedCut> {
        self.0.detect(context)
    }
}
