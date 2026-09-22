use crate::framework::{FallThrough, FallThroughContext};
use crate::{IndexedConcurrencyGraph, MinerError, Node, OperatorType};

/// Guaranteed lifecycle fallback covering simultaneous executions.
/// For each activity, creates as many parallel optional repeatable branches as
/// its maximum observed execution concurrency. Empty input returns tau.
#[derive(Debug, Clone, Copy, Default)]
pub struct ConcurrentFlowerModel;
impl FallThrough for ConcurrentFlowerModel {
    fn fall_through(&self, c: &FallThroughContext<'_>) -> Result<Node, MinerError> {
        let graph = IndexedConcurrencyGraph::from_log(c.log);
        let mut activities: Vec<_> = graph.max_concurrency.into_iter().collect();
        activities.sort_by_key(|(a, _)| *a);
        let mut branches = Vec::new();
        for (a, count) in activities {
            for _ in 0..count {
                branches.push(Node::new_operator_with_children(
                    OperatorType::Loop,
                    vec![
                        Node::new_leaf(None),
                        Node::new_leaf(Some(c.log.resolve(a).clone())),
                    ],
                ));
            }
        }
        Ok(match branches.len() {
            0 => Node::new_leaf(None),
            1 => branches.pop().unwrap(),
            _ => Node::new_operator_with_children(OperatorType::Concurrent, branches),
        })
    }
}
