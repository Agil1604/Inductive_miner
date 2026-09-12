use super::FallThroughStrategy;
use crate::components::filtering::FilteringConfig;
use crate::framework::FallThroughContext;
use crate::{MinerError, Node, OperatorType};

/// Handles empty traces according to their observed frequency.
///
/// Removes empty traces and recursively mines the nonempty sublog using the
/// current miner's callback. If `empty_count >= trace_count * threshold`, returns
/// exclusive choice between a silent leaf and the mined node. Equality retains
/// optionality. Otherwise, returns only the mined node, filtering out the empty
/// behaviour. Duplicate traces count separately in this comparison.
///
/// Returns `None` when no empty traces occur, including a log with no traces.
/// A log consisting entirely of empty traces returns a silent leaf directly.
/// The recursive sublog preserves case IDs, events, and the original interner.
/// Filtering may remove observed empty behaviour and
/// therefore does not guarantee fitness.
#[derive(Debug, Clone, Copy, Default)]
pub struct EmptyTracesFiltering {
    /// Minimum empty-trace fraction required to retain optionality.
    /// The default threshold is `0.2`; zero retains any observed empty traces.
    pub config: FilteringConfig,
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
