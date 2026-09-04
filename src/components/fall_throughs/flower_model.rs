use crate::framework::{FallThrough, FallThroughContext};
use crate::{MinerError, Node, OperatorType};

///
/// Guaranteed fallback allowing any nonempty sequence of observed activities.
/// An input without activities returns a silent leaf.
///
#[derive(Debug, Clone, Copy, Default)]
pub struct FlowerModel;
impl FallThrough for FlowerModel {
    fn fall_through(&self, context: &FallThroughContext<'_>) -> Result<Node, MinerError> {
        let log = context.log;
        let leaves: Vec<_> = log
            .alphabet()
            .into_iter()
            .map(|a| Node::new_leaf(Some(log.resolve(a).clone())))
            .collect();
        if leaves.is_empty() {
            return Ok(Node::new_leaf(None));
        }
        let flower = Node::new_operator_with_children(
            OperatorType::Loop,
            vec![
                Node::new_operator_with_children(OperatorType::Xor, leaves),
                Node::new_leaf(None),
            ],
        );
        if log.traces.iter().any(|t| t.events.is_empty()) {
            Ok(Node::new_operator_with_children(
                OperatorType::Xor,
                vec![Node::new_leaf(None), flower],
            ))
        } else {
            Ok(flower)
        }
    }
}
