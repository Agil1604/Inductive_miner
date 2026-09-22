use super::super::common::{components, cut};
use super::{NonAtomicCutContext, NonAtomicCutStrategy};
use crate::{IndexedCut, OperatorType};
use std::collections::HashSet;

/// Detects [`OperatorType::Loop`] cuts for IMlc.
///
/// Initializes the body with all observed start and end activities. Remaining
/// activities form redo candidates connected by directly-follows edges in
/// either direction or by execution overlap.
///
/// Candidates are absorbed into the body when they overlap it, have an edge
/// from a start-only activity or to an end-only activity, or connect to some
/// but not all required start/end activities. Overlap checks continue as the
/// body grows, keeping overlapping activities in the same partition.
///
/// The first partition is the body; subsequent partitions are redo alternatives.
/// Returns `None` if no body or no separate redo part remains.
#[derive(Debug, Clone, Copy, Default)]
pub struct NonAtomicLoopCut;
impl NonAtomicCutStrategy for NonAtomicLoopCut {
    fn detect(&self, c: &NonAtomicCutContext<'_>) -> Option<IndexedCut> {
        let g = &c.dfg;
        let mut body: HashSet<_> = g
            .start_activities
            .keys()
            .chain(g.end_activities.keys())
            .copied()
            .collect();
        if body.is_empty() {
            return None;
        }
        let rest: Vec<_> = c
            .activities
            .iter()
            .copied()
            .filter(|a| !body.contains(a))
            .collect();
        let redo = components(&rest, |i, j| {
            c.edge(rest[i], rest[j]) || c.edge(rest[j], rest[i]) || c.overlap(rest[i], rest[j])
        });
        let mut retained = Vec::new();
        for p in redo {
            let invalid = p.iter().any(|a| {
                body.iter().any(|b| c.overlap(*a, *b))
                    || g.start_activities
                        .keys()
                        .any(|s| !g.end_activities.contains_key(s) && c.edge(*s, *a))
                    || g.end_activities
                        .keys()
                        .any(|e| !g.start_activities.contains_key(e) && c.edge(*a, *e))
                    || (g.start_activities.keys().any(|s| c.edge(*a, *s))
                        && !g.start_activities.keys().all(|s| c.edge(*a, *s)))
                    || (g.end_activities.keys().any(|e| c.edge(*e, *a))
                        && !g.end_activities.keys().all(|e| c.edge(*e, *a)))
            });
            if invalid {
                body.extend(p);
            } else {
                retained.push(p);
            }
        }

        loop {
            let Some(i) = retained
                .iter()
                .position(|p| p.iter().any(|a| body.iter().any(|b| c.overlap(*a, *b))))
            else {
                break;
            };
            body.extend(retained.remove(i));
        }
        let mut parts = vec![body];
        parts.extend(retained);
        cut(OperatorType::Loop, parts)
    }
}
