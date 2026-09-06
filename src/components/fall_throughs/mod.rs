//! Reusable optional fall-through strategies and guaranteed fallbacks.
mod activity_concurrent;
mod activity_once_per_trace;
mod common;
mod empty_traces;
mod empty_traces_filtering;
mod flower_model;
mod strict_tau_loop;
mod tau_loop;

pub use activity_concurrent::ActivityConcurrent;
pub use activity_once_per_trace::ActivityOncePerTrace;
pub use empty_traces::EmptyTraces;
pub use empty_traces_filtering::EmptyTracesFiltering;
pub use flower_model::FlowerModel;
pub use strict_tau_loop::StrictTauLoop;
pub use tau_loop::TauLoop;

use crate::framework::{FallThrough, FallThroughContext};
use crate::{MinerError, Node};

/// An optional strategy. `None` means that the next strategy should be tried.
pub trait FallThroughStrategy {
    fn try_apply(&self, context: &FallThroughContext<'_>) -> Result<Option<Node>, MinerError>;
}

/// First-match composition, propagating errors without trying later strategies.
#[derive(Debug, Clone, Copy, Default)]
pub struct FallThroughChain<A, B> {
    pub first: A,
    pub second: B,
}
impl<A: FallThroughStrategy, B: FallThroughStrategy> FallThroughStrategy
    for FallThroughChain<A, B>
{
    fn try_apply(&self, context: &FallThroughContext<'_>) -> Result<Option<Node>, MinerError> {
        match self.first.try_apply(context)? {
            Some(node) => Ok(Some(node)),
            None => self.second.try_apply(context),
        }
    }
}

/// Adapts optional strategies to the framework with a guaranteed fallback.
#[derive(Debug, Clone, Copy, Default)]
pub struct FallThroughFinder<S, F> {
    pub strategies: S,
    pub fallback: F,
}
impl<S: FallThroughStrategy, F: FallThrough> FallThrough for FallThroughFinder<S, F> {
    fn fall_through(&self, context: &FallThroughContext<'_>) -> Result<Node, MinerError> {
        match self.strategies.try_apply(context)? {
            Some(node) => Ok(node),
            None => self.fallback.fall_through(context),
        }
    }
}

#[cfg(test)]
mod tests;
