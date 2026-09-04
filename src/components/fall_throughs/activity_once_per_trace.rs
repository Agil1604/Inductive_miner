use super::FallThroughStrategy;
use crate::components::log_splitting::project;
use crate::framework::FallThroughContext;
use crate::{MinerError, Node, OperatorType};

///
/// Extracts the first lexical activity occurring exactly once in every trace
/// and mines the remaining events as a concurrent branch.
///
#[derive(Debug, Clone, Copy, Default)]
pub struct ActivityOncePerTrace;
impl FallThroughStrategy for ActivityOncePerTrace {
    fn try_apply(&self, context: &FallThroughContext<'_>) -> Result<Option<Node>, MinerError> {
        let log = context.log;
        for a in log.alphabet() {
            if log
                .traces
                .iter()
                .all(|t| t.events.iter().filter(|e| e.activity == a).count() == 1)
            {
                let other = project(log, |b| *b != a);
                return Ok(Some(Node::new_operator_with_children(
                    OperatorType::Concurrent,
                    vec![
                        Node::new_leaf(Some(log.resolve(a).clone())),
                        (context.recurse)(&other)?,
                    ],
                )));
            }
        }
        Ok(None)
    }
}
