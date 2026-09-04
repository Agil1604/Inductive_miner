use super::common::{components, cut};
use super::{CutContext, CutStrategy};
use crate::{ActivityId, IndexedCut, OperatorType};
use std::collections::HashSet;

///
/// Detects loop cuts with the body first and alternative redo parts afterward.
/// Returns `None` when fewer than two parts can be found.
///
#[derive(Debug, Clone, Copy, Default)]
pub struct LoopCut;
impl CutStrategy for LoopCut {
    fn detect(&self, context: &CutContext<'_>) -> Option<IndexedCut> {
        let a = context.activities();
        let g = context.dfg();
        let edge = |x: &ActivityId, y: &ActivityId| g.edges.contains_key(&(*x, *y));
        let mut body: HashSet<_> = g
            .start_activities
            .keys()
            .chain(g.end_activities.keys())
            .cloned()
            .collect();
        let rest: Vec<_> = a.iter().filter(|a| !body.contains(*a)).cloned().collect();
        let redo = components(&rest, |i, j| {
            edge(&rest[i], &rest[j]) || edge(&rest[j], &rest[i])
        });
        let mut retained = Vec::new();
        for p in redo {
            let invalid = p.iter().any(|a| {
                g.start_activities
                    .keys()
                    .any(|s| !g.end_activities.contains_key(s) && edge(s, a))
                    || g.end_activities
                        .keys()
                        .any(|e| !g.start_activities.contains_key(e) && edge(a, e))
                    || (g.start_activities.keys().any(|s| edge(a, s))
                        && !g.start_activities.keys().all(|s| edge(a, s)))
                    || (g.end_activities.keys().any(|e| edge(e, a))
                        && !g.end_activities.keys().all(|e| edge(e, a)))
            });
            if invalid {
                body.extend(p);
            } else {
                retained.push(p);
            }
        }
        let mut parts = vec![body];
        parts.extend(retained);
        cut(OperatorType::Loop, parts)
    }
}
