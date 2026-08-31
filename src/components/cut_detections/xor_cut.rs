use super::common::{components, cut};
use super::{CutContext, CutStrategy};
use crate::{ActivityId, IndexedCut, OperatorType};

///
/// Detects exclusive-choice cuts using weakly connected DFG components.
/// Returns `None` when fewer than two parts can be found.
///
#[derive(Debug, Clone, Copy, Default)]
pub struct XorCut;
impl CutStrategy for XorCut {
    fn detect(&self, context: &CutContext<'_>) -> Option<IndexedCut> {
        let a = context.activities();
        let g = context.dfg();
        let edge = |x: &ActivityId, y: &ActivityId| g.edges.contains_key(&(*x, *y));
        cut(
            OperatorType::Xor,
            components(a, |i, j| edge(&a[i], &a[j]) || edge(&a[j], &a[i])),
        )
    }
}
