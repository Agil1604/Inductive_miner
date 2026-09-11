use super::common::{components, cut};
use super::{CutContext, CutStrategy};
use crate::{IndexedCut, OperatorType};

/// Detects [`OperatorType::Interleaved`]
///
/// Across distinct parts, a directly-follows edge must exist exactly when its
/// source is an observed end activity and its target an observed start activity.
/// Activities violating this rule, or connected by minimum self-distance
/// witnesses, are merged into the same part.
///
/// A further pass through the original traces merges parts whenever execution
/// returns to a part after visiting another one. Merging and rescanning continue
/// until every part occupies at most one contiguous block per trace.
///
/// Returns `None` if these checks leave fewer than two parts. Returned parts
/// cover the context's activity alphabet and can be split with
/// [`crate::components::log_splitting::InterleavedSplit`].
#[derive(Debug, Default, Clone, Copy)]
pub struct InterleavedCut;
impl CutStrategy for InterleavedCut {
    fn detect(&self, context: &CutContext<'_>) -> Option<IndexedCut> {
        let a = context.activities();
        let g = context.dfg();
        let w = context.witnesses();
        let incompatible = |i: usize, j: usize| {
            let (x, y) = (a[i], a[j]);
            let edge = g.edges.contains_key(&(x, y));
            let expected = g.end_activities.contains_key(&x) && g.start_activities.contains_key(&y);
            edge != expected || w.contains(&(x, y))
        };
        let mut parts = components(a, |i, j| incompatible(i, j) || incompatible(j, i));
        loop {
            let mut merge = None;
            'traces: for trace in &context.log().traces {
                let mut runs = Vec::new();
                for event in &trace.events {
                    let i = parts
                        .iter()
                        .position(|p| p.contains(&event.activity))
                        .unwrap();
                    if runs.last() == Some(&i) {
                        continue;
                    }
                    if let Some(start) = runs.iter().position(|&p| p == i) {
                        merge = Some((i, runs[start + 1]));
                        break 'traces;
                    }
                    runs.push(i);
                }
            }
            let Some((i, j)) = merge else {
                break;
            };
            let (keep, remove) = (i.min(j), i.max(j));
            let moved = parts.remove(remove);
            parts[keep].extend(moved);
        }
        cut(OperatorType::Interleaved, parts)
    }
}
