use super::FallThroughStrategy;
use crate::components::log_splitting::project;
use crate::framework::FallThroughContext;
use crate::{MinerError, Node, OperatorType};

///
/// Extracts an activity whose removal enables a cut, then mines both parallel
/// branches. Uses the current miner's cut finder and recursion callbacks.
///
#[derive(Debug, Clone, Copy, Default)]
pub struct ActivityConcurrent;
impl FallThroughStrategy for ActivityConcurrent {
    fn try_apply(&self, context: &FallThroughContext<'_>) -> Result<Option<Node>, MinerError> {
        let log = context.log;
        if log.traces.iter().any(|t| t.events.is_empty()) {
            return Ok(None);
        }
        for a in log.alphabet() {
            let other = project(log, |b| *b != a);
            if (context.find_cut)(&other).is_some() {
                let activity = project(log, |b| *b == a);
                return Ok(Some(Node::new_operator_with_children(
                    OperatorType::Concurrent,
                    vec![(context.recurse)(&activity)?, (context.recurse)(&other)?],
                )));
            }
        }
        Ok(None)
    }
}
