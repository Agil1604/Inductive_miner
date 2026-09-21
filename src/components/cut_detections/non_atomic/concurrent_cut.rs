use super::super::common::{components, cut};
use super::{NonAtomicCutContext, NonAtomicCutStrategy};
use crate::{IndexedCut, OperatorType};

/// Detects [`OperatorType::Concurrent`] cuts for IMlc.
///
/// Activities in different partitions must have either directly-follows edges
/// in both directions or an observed execution overlap. Pairs lacking both
/// forms of evidence are merged. Partitions without an observed start or end
/// activity are then merged until every remaining part has both boundaries.
///
/// Uses the graphs in [`NonAtomicCutContext`] without atomic minimum
/// self-distance witnesses. Returns `None` when fewer than two parts remain.
#[derive(Debug, Clone, Copy, Default)]
pub struct NonAtomicConcurrentCut;
impl NonAtomicCutStrategy for NonAtomicConcurrentCut {
    fn detect(&self, c: &NonAtomicCutContext<'_>) -> Option<IndexedCut> {
        let a = &c.activities;
        let mut parts = components(a, |i, j| {
            !c.overlap(a[i], a[j]) && (!c.edge(a[i], a[j]) || !c.edge(a[j], a[i]))
        });
        while parts.len() > 1 {
            let Some(i) = parts.iter().position(|p| {
                !p.iter().any(|a| c.dfg.start_activities.contains_key(a))
                    || !p.iter().any(|a| c.dfg.end_activities.contains_key(a))
            }) else {
                break;
            };
            let moved = parts.remove(i);
            parts[if i == 0 { 0 } else { i - 1 }].extend(moved);
        }
        cut(OperatorType::Concurrent, parts)
    }
}
