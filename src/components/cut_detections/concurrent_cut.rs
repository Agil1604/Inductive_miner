use super::common::{components, cut};
use super::{CutContext, CutStrategy};
use crate::{ActivityId, IndexedCut, OperatorType};

///
/// Detects concurrency cuts using bidirectional edges, start/end activities,
/// and minimum self-distance witnesses.
/// Returns `None` when fewer than two parts can be found.
///
#[derive(Debug, Clone, Copy, Default)]
pub struct ConcurrentCut;
impl CutStrategy for ConcurrentCut {
    fn detect(&self, context: &CutContext<'_>) -> Option<IndexedCut> {
        let a = context.activities();
        let g = context.dfg();
        let w = context.witnesses();
        let edge = |x: &ActivityId, y: &ActivityId| g.edges.contains_key(&(*x, *y));
        let mut parts = components(a, |i, j| {
            !edge(&a[i], &a[j])
                || !edge(&a[j], &a[i])
                || w.contains(&(a[i], a[j]))
                || w.contains(&(a[j], a[i]))
        });

        while parts.len() > 1 {
            let bad = parts.iter().position(|p| {
                !p.iter().any(|a| g.start_activities.contains_key(a))
                    || !p.iter().any(|a| g.end_activities.contains_key(a))
            });
            let Some(i) = bad else {
                break;
            };
            let p = parts.remove(i);
            let target = if i == 0 { 0 } else { i - 1 };
            parts[target].extend(p);
        }
        cut(OperatorType::Concurrent, parts)
    }
}
