use super::{NonAtomicCutContext, NonAtomicCutStrategy};
use crate::components::cut_detections::filter_dfg;
use crate::components::filtering::FilteringConfig;
use crate::framework::DetectCut;
use crate::{IndexedCut, IndexedEventLog};

/// Attempts lifecycle cuts before filtering the DFG and trying a fallback chain.
/// Original overlap evidence is retained in both passes (§6.5.3).
#[derive(Debug, Clone, Copy, Default)]
pub struct NonAtomicFilteringCutFinder<U, F> {
    /// Unfiltered first-pass strategies.
    pub unfiltered: U,
    /// Strategies applied to the filtered DFG when the first pass fails.
    pub filtered: F,
    /// Shared DFG filtering threshold.
    pub config: FilteringConfig,
}
impl<U: NonAtomicCutStrategy, F: NonAtomicCutStrategy> DetectCut
    for NonAtomicFilteringCutFinder<U, F>
{
    fn detect_cut(&self, log: &IndexedEventLog) -> Option<IndexedCut> {
        let context = NonAtomicCutContext::new(log)?;
        self.unfiltered.detect(&context).or_else(|| {
            self.filtered
                .detect(&context.with_dfg(filter_dfg(&context.dfg, self.config)))
        })
    }
}
