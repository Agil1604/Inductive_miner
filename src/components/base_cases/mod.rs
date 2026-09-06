mod empty_log;
mod single_activity;
mod single_activity_filtering;

pub use empty_log::EmptyLog;
pub use single_activity::SingleActivity;
pub use single_activity_filtering::SingleActivityFiltering;

use crate::framework::BaseCase;
use crate::{IndexedEventLog, Node};

/// Applies a base case only when no input trace is empty. This lets a
/// fall-through decide whether to retain or filter optionality.
#[derive(Debug, Clone, Copy, Default)]
pub struct WithoutEmptyTraces<B>(pub B);

impl<B: BaseCase> BaseCase for WithoutEmptyTraces<B> {
    fn base_case(&self, log: &IndexedEventLog) -> Option<Node> {
        if log.traces.iter().any(|t| t.events.is_empty()) {
            None
        } else {
            self.0.base_case(log)
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BaseCaseChain<A, B> {
    pub first: A,
    pub second: B,
}

impl<A: BaseCase, B: BaseCase> BaseCase for BaseCaseChain<A, B> {
    fn base_case(&self, log: &IndexedEventLog) -> Option<Node> {
        self.first
            .base_case(log)
            .or_else(|| self.second.base_case(log))
    }
}
