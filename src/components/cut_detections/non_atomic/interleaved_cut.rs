use super::super::common::{components, cut};
use super::{NonAtomicCutContext, NonAtomicCutStrategy};
use crate::Lifecycle;
use crate::{IndexedCut, OperatorType};

/// Detects [`OperatorType::Interleaved`] cuts for IMlc.
///
/// Across distinct parts, a directly-follows edge must exist exactly when its
/// source is an observed end activity and its target an observed start activity.
/// Activities violating this footprint are merged, without atomic minimum
/// self-distance witness checks.
///
/// Scans start, completion, and unknown events in the original traces, merging
/// and rescanning whenever execution returns to an earlier part. Every final
/// part occupies at most one contiguous block of relevant events per trace;
/// this also prevents overlapping executions from being split across parts.
/// Enqueue and other transitions are ignored.
///
/// Returns `None` when fewer than two parts remain.
#[derive(Debug, Clone, Copy, Default)]
pub struct NonAtomicInterleavedCut;
impl NonAtomicCutStrategy for NonAtomicInterleavedCut {
    fn detect(&self, c: &NonAtomicCutContext<'_>) -> Option<IndexedCut> {
        let a = &c.activities;
        let incompatible = |x, y| {
            c.edge(x, y)
                != (c.dfg.end_activities.contains_key(&x)
                    && c.dfg.start_activities.contains_key(&y))
        };
        let mut parts = components(a, |i, j| {
            incompatible(a[i], a[j]) || incompatible(a[j], a[i])
        });
        loop {
            let mut merge = None;
            'traces: for t in &c.log.traces {
                let mut runs = Vec::new();
                for e in &t.events {
                    let Some(i) = parts.iter().position(|p| p.contains(&e.activity)) else {
                        continue;
                    };
                    if !matches!(
                        e.lifecycle,
                        Lifecycle::Start | Lifecycle::Complete | Lifecycle::Unknown
                    ) || runs.last() == Some(&i)
                    {
                        continue;
                    }
                    if let Some(k) = runs.iter().position(|p| *p == i) {
                        merge = Some((i, runs[k + 1]));
                        break 'traces;
                    }
                    runs.push(i);
                }
            }
            let Some((i, j)) = merge else {
                break;
            };
            let moved = parts.remove(i.max(j));
            parts[i.min(j)].extend(moved);
        }
        cut(OperatorType::Interleaved, parts)
    }
}
