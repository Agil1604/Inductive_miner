use super::FallThroughStrategy;
use crate::algorithms::imf::ImfConfig;
use crate::framework::FallThroughContext;
use crate::{MinerError, Node, OperatorType};

///
/// Empty-trace filtering fall-through strategy.
///
#[derive(Debug, Clone, Copy, Default)]
pub struct EmptyTracesFiltering {
    pub config: ImfConfig,
}

impl FallThroughStrategy for EmptyTracesFiltering {
    fn try_apply(&self, context: &FallThroughContext<'_>) -> Result<Option<Node>, MinerError> {
        let log = context.log;
        let empty_count = log.traces.iter().filter(|t| t.events.is_empty()).count();
        if empty_count == 0 {
            return Ok(None);
        }

        if empty_count == log.traces.len() {
            return Ok(Some(Node::new_leaf(None)));
        }
        let nonempty = log.with_traces(
            log.traces
                .iter()
                .filter(|t| !t.events.is_empty())
                .cloned()
                .collect(),
        );
        let child = (context.recurse)(&nonempty)?;
        if empty_count as f64 >= log.traces.len() as f64 * self.config.deviation_threshold() {
            Ok(Some(Node::new_operator_with_children(
                OperatorType::Xor,
                vec![Node::new_leaf(None), child],
            )))
        } else {
            Ok(Some(child))
        }
    }
}
