mod empty_log;
mod single_activity;

pub use empty_log::EmptyLog;
pub use single_activity::SingleActivity;

use crate::framework::BaseCase;
use crate::{EventLog, Node};

#[derive(Debug, Clone, Copy, Default)]
pub struct BaseCaseChain<A, B> {
    pub first: A,
    pub second: B,
}

impl<A: BaseCase, B: BaseCase> BaseCase for BaseCaseChain<A, B> {
    fn base_case(&self, log: &EventLog) -> Option<Node> {
        self.first
            .base_case(log)
            .or_else(|| self.second.base_case(log))
    }
}
