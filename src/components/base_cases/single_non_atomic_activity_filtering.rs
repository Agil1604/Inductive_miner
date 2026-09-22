use crate::components::base_cases::SingleActivityFiltering;
use crate::components::filtering::FilteringConfig;
use crate::components::log_splitting::repair_consistency;
use crate::framework::BaseCase;
use crate::{IndexedEventLog, Node};

/// Applies IMf's single-activity estimate to lifecycle execution counts.
/// Repairs unmatched starts before projecting completions and unknown atomic
/// events. Matches when `abs(traces / (executions + traces) - 0.5) <= threshold`.
#[derive(Debug, Clone, Copy, Default)]
pub struct SingleNonAtomicActivityFiltering {
    /// Shared deviation threshold.
    pub config: FilteringConfig,
}
impl BaseCase for SingleNonAtomicActivityFiltering {
    fn base_case(&self, log: &IndexedEventLog) -> Option<Node> {
        let repaired = repair_consistency(log);
        let atomic = repaired.atomic();
        let projected = log.with_traces(atomic.traces);
        SingleActivityFiltering {
            config: self.config,
        }
        .base_case(&projected)
    }
}
