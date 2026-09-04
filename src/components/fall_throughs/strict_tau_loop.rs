use super::{FallThroughStrategy, common};
use crate::framework::FallThroughContext;
use crate::{MinerError, Node};

/// Splits before a start activity only when preceded by an end activity.
/// Mines the segments as a loop body with a silent redo when a split occurred.
#[derive(Debug, Clone, Copy, Default)]
pub struct StrictTauLoop;
impl FallThroughStrategy for StrictTauLoop {
    fn try_apply(&self, context: &FallThroughContext<'_>) -> Result<Option<Node>, MinerError> {
        common::tau_loop(context, true)
    }
}
