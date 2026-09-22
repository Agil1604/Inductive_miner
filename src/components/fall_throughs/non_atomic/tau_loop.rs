use crate::components::fall_throughs::FallThroughStrategy;
use crate::framework::FallThroughContext;
use crate::{IndexedDfg, Lifecycle, MinerError, Node, OperatorType, Trace};
use std::collections::HashMap;

/// Splits before a subsequent start activity only after all open executions close.
/// Mines the segments as a loop body with silent redo (thesis §6.5.2).
#[derive(Debug, Clone, Copy, Default)]
pub struct NonAtomicTauLoop;

/// Like [`NonAtomicTauLoop`], also requires a preceding end-activity completion.
#[derive(Debug, Clone, Copy, Default)]
pub struct StrictNonAtomicTauLoop;

impl FallThroughStrategy for NonAtomicTauLoop {
    fn try_apply(&self, c: &FallThroughContext<'_>) -> Result<Option<Node>, MinerError> {
        split(c, false)
    }
}
impl FallThroughStrategy for StrictNonAtomicTauLoop {
    fn try_apply(&self, c: &FallThroughContext<'_>) -> Result<Option<Node>, MinerError> {
        split(c, true)
    }
}
fn split(c: &FallThroughContext<'_>, strict: bool) -> Result<Option<Node>, MinerError> {
    let graph = IndexedDfg::from_non_atomic_log(c.log);
    if graph.empty_trace_count > 0 {
        return Ok(None);
    }
    let mut traces = Vec::new();
    for t in &c.log.traces {
        let mut active = HashMap::new();
        let mut start = 0;
        let mut previous = None;
        for (i, e) in t.events.iter().enumerate() {
            if matches!(e.lifecycle, Lifecycle::Enqueue | Lifecycle::Other) {
                continue;
            }
            let running = active.get(&e.activity).copied().unwrap_or(0usize);
            let begins = e.lifecycle == Lifecycle::Start
                || e.lifecycle == Lifecycle::Unknown
                || (e.lifecycle == Lifecycle::Complete && running == 0);
            if i > start
                && begins
                && active.is_empty()
                && graph.start_activities.contains_key(&e.activity)
                && (!strict || previous.is_some_and(|a| graph.end_activities.contains_key(&a)))
            {
                traces.push(Trace {
                    case_id: t.case_id.clone(),
                    events: t.events[start..i].to_vec(),
                });
                start = i;
            }
            match e.lifecycle {
                Lifecycle::Start => {
                    *active.entry(e.activity).or_insert(0usize) += 1;
                    previous = None;
                }
                Lifecycle::Complete => {
                    if running > 1 {
                        active.insert(e.activity, running - 1);
                    } else {
                        active.remove(&e.activity);
                    }
                    previous = Some(e.activity);
                }
                Lifecycle::Unknown => previous = Some(e.activity),
                _ => {}
            }
        }
        traces.push(Trace {
            case_id: t.case_id.clone(),
            events: t.events[start..].to_vec(),
        });
    }
    if traces.len() <= c.log.traces.len() {
        return Ok(None);
    }
    let split = c.log.with_traces(traces);
    Ok(Some(Node::new_operator_with_children(
        OperatorType::Loop,
        vec![(c.recurse)(&split)?, Node::new_leaf(None)],
    )))
}
