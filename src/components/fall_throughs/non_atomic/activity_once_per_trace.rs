use crate::components::base_cases::SingleNonAtomicActivity;
use crate::components::fall_throughs::FallThroughStrategy;
use crate::components::log_splitting::project;
use crate::framework::{BaseCase, FallThroughContext};
use crate::{MinerError, Node, OperatorType};

/// Extracts the first activity with exactly one lifecycle execution per trace.
/// Accepts start/completion pairs and atomic completion or unknown events.
/// Mines the remaining activities as a concurrent branch.
#[derive(Debug, Clone, Copy, Default)]
pub struct NonAtomicActivityOncePerTrace;
impl FallThroughStrategy for NonAtomicActivityOncePerTrace {
    fn try_apply(&self, c: &FallThroughContext<'_>) -> Result<Option<Node>, MinerError> {
        for a in c.log.alphabet() {
            let activity = project(c.log, |b| *b == a);
            if let Some(leaf) = SingleNonAtomicActivity.base_case(&activity) {
                let other = project(c.log, |b| *b != a);
                return Ok(Some(Node::new_operator_with_children(
                    OperatorType::Concurrent,
                    vec![leaf, (c.recurse)(&other)?],
                )));
            }
        }
        Ok(None)
    }
}
