use super::common::cut;
use super::{CutContext, CutStrategy, SequenceCut};
use crate::{IndexedCut, OperatorType};
use std::collections::HashMap;

/// Refines [`SequenceCut`] for optional behaviour in IMa (thesis §6.4.2).
///
/// Starts with the ordinary ordered sequence partition. A part is skippable if
/// an edge jumps across it, a trace can start after it, or a trace can end before
/// it. For each such pivot, predecessor and successor edge bounds determine a
/// scope of neighbouring parts to merge. Keeping these parts together allows
/// recursion to discover optionality around their combined behaviour.
///
/// Returns an ordered [`OperatorType::Sequence`] cut, or `None` when no initial
/// sequence cut exists or all parts merge into one. Use the standard
/// [`crate::components::log_splitting::SequenceSplit`] with the returned cut.
#[derive(Debug, Default, Clone, Copy)]
pub struct SequenceCutStrict;
impl CutStrategy for SequenceCutStrict {
    fn detect(&self, context: &CutContext<'_>) -> Option<IndexedCut> {
        let mut parts = SequenceCut.detect(context)?.partitions;
        let n = parts.len();
        let mut owners: Vec<_> = (0..n).collect();
        let indices: HashMap<_, _> = parts
            .iter()
            .enumerate()
            .flat_map(|(i, p)| p.iter().map(move |&a| (a, i)))
            .collect();
        let g = context.dfg();
        let mut min_from = vec![n as isize; n];
        let mut max_to = vec![-1isize; n];
        for a in g.start_activities.keys() {
            min_from[indices[a]] = -1;
        }
        for a in g.end_activities.keys() {
            max_to[indices[a]] = n as isize;
        }
        for &(a, b) in g.edges.keys() {
            let (i, j) = (indices[&a], indices[&b]);
            min_from[j] = min_from[j].min(i as isize);
            max_to[i] = max_to[i].max(j as isize);
        }
        for p in 0..n {
            let skippable = g
                .edges
                .keys()
                .any(|(a, b)| indices[a] < p && indices[b] > p)
                || g.start_activities.keys().any(|a| indices[a] > p)
                || g.end_activities.keys().any(|a| indices[a] < p);
            if !skippable {
                continue;
            }
            let mut q = p;
            while q > 0 && max_to[q - 1] <= p as isize {
                q -= 1;
                merge(&mut parts, &mut owners, q, p);
            }
            q = p + 1;
            while q < n && min_from[q] >= p as isize {
                merge(&mut parts, &mut owners, q, p);
                q += 1;
            }
        }
        parts.retain(|p| !p.is_empty());
        cut(OperatorType::Sequence, parts)
    }
}

fn merge(
    parts: &mut [std::collections::HashSet<crate::ActivityId>],
    owners: &mut [usize],
    a: usize,
    b: usize,
) {
    let (a, b) = (owners[a], owners[b]);
    if a == b {
        return;
    }
    let (keep, remove) = (a.min(b), a.max(b));
    let moved = std::mem::take(&mut parts[remove]);
    parts[keep].extend(moved);
    for owner in owners {
        if *owner == remove {
            *owner = keep;
        }
    }
}
