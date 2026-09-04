use crate::framework::FallThroughContext;
use crate::{IndexedDfg, MinerError, Node, OperatorType, Trace};

///
/// Splits before each subsequent start activity.
/// Mines the segments as a loop body with a silent redo when a split occurred.
///
pub(super) fn tau_loop(
    context: &FallThroughContext<'_>,
    strict: bool,
) -> Result<Option<Node>, MinerError> {
    let log = context.log;
    if log.traces.iter().any(|t| t.events.is_empty()) {
        return Ok(None);
    }
    let graph = IndexedDfg::from_log(log);
    let mut traces = Vec::new();
    for trace in &log.traces {
        let mut start = 0;
        for i in 1..trace.events.len() {
            if graph
                .start_activities
                .contains_key(&trace.events[i].activity)
                && (!strict
                    || graph
                        .end_activities
                        .contains_key(&trace.events[i - 1].activity))
            {
                traces.push(Trace {
                    case_id: trace.case_id.clone(),
                    events: trace.events[start..i].to_vec(),
                });
                start = i;
            }
        }
        traces.push(Trace {
            case_id: trace.case_id.clone(),
            events: trace.events[start..].to_vec(),
        });
    }
    if traces.len() <= log.traces.len() {
        return Ok(None);
    }
    let split = log.with_traces(traces);
    Ok(Some(Node::new_operator_with_children(
        OperatorType::Loop,
        vec![(context.recurse)(&split)?, Node::new_leaf(None)],
    )))
}
