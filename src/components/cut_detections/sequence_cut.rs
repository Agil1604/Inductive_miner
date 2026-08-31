use super::common::{components, cut};
use super::{CutContext, CutStrategy};
use crate::{ActivityId, IndexedCut, OperatorType};
use std::collections::HashMap;

///
/// Detects ordered sequence cuts using DFG reachability.
/// Returns `None` when fewer than two parts can be found.
///
#[derive(Debug, Clone, Copy, Default)]
pub struct SequenceCut;
impl CutStrategy for SequenceCut {
    fn detect(&self, context: &CutContext<'_>) -> Option<IndexedCut> {
        let a = context.activities();
        let g = context.dfg();
        let edge = |x: &ActivityId, y: &ActivityId| g.edges.contains_key(&(*x, *y));
        let mut reach = vec![vec![false; a.len()]; a.len()];
        for i in 0..a.len() {
            for j in 0..a.len() {
                reach[i][j] = edge(&a[i], &a[j]);
            }
        }
        for k in 0..a.len() {
            for i in 0..a.len() {
                for j in 0..a.len() {
                    reach[i][j] |= reach[i][k] && reach[k][j];
                }
            }
        }
        let mut parts = components(a, |i, j| reach[i][j] == reach[j][i]);
        if parts.len() > 1 {
            let index: HashMap<_, _> = a.iter().enumerate().map(|(i, a)| (a, i)).collect();
            parts.sort_by(|p, q| {
                let x = index[p.iter().min().unwrap()];
                let y = index[q.iter().min().unwrap()];
                if x == y {
                    std::cmp::Ordering::Equal
                } else if reach[x][y] {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                }
            });
            return cut(OperatorType::Sequence, parts);
        }
        None
    }
}
