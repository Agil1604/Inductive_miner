mod concurrent_cut;
mod context;
mod finder;
mod interleaved_cut;
mod loop_cut;

pub use concurrent_cut::NonAtomicConcurrentCut;
pub use context::NonAtomicCutContext;
pub use finder::{NonAtomicCutFinder, NonAtomicDfgCut};
pub use interleaved_cut::NonAtomicInterleavedCut;
pub use loop_cut::NonAtomicLoopCut;

use crate::IndexedCut;

/// A reusable detector borrowing prepared lifecycle graphs.
pub trait NonAtomicCutStrategy {
    /// Returns a cut with at least two parts, or `None`.
    fn detect(&self, context: &NonAtomicCutContext<'_>) -> Option<IndexedCut>;
}

#[cfg(test)]
mod tests;
