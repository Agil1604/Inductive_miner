use super::FallThroughStrategy;
use crate::framework::FallThroughContext;
use crate::{MinerError, Node, OperatorType};

///
/// Accounts for empty traces with XOR(tau, recursively mined nonempty log).
///
#[derive(Debug, Clone, Copy, Default)]
pub struct EmptyTraces;
impl FallThroughStrategy for EmptyTraces {
    fn try_apply(&self, context: &FallThroughContext<'_>) -> Result<Option<Node>, MinerError> {
        let log = context.log;
        if !log.traces.iter().any(|t| t.events.is_empty()) {
            return Ok(None);
        }
        let nonempty = log.with_traces(
            log.traces
                .iter()
                .filter(|t| !t.events.is_empty())
                .cloned()
                .collect(),
        );
        Ok(Some(Node::new_operator_with_children(
            OperatorType::Xor,
            vec![Node::new_leaf(None), (context.recurse)(&nonempty)?],
        )))
    }
}
