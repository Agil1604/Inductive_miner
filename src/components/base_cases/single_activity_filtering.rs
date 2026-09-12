use crate::components::filtering::FilteringConfig;
use crate::framework::BaseCase;
use crate::{IndexedEventLog, Node};

/// Returns an activity leaf when a single-activity log is sufficiently close
/// to one execution per trace.
///
/// For a log with exactly one distinct activity, estimates the geometric
/// parameter `p = trace_count / (event_count + trace_count)`. A log with one
/// event per trace has `p = 0.5`. The base case matches when
/// `abs(p - 0.5) <= config.deviation_threshold()`, including equality.
/// Repeated executions can therefore be simplified to a single activity leaf.
///
/// This component includes empty traces in the estimate. 
/// Logs with no activities or more than one distinct activity return `None`.
/// Returned leaves resolve the interned activity ID to its original name.
/// Filtering may discard observed behaviour or introduce a single execution
/// not present in the log.
#[derive(Debug, Clone, Copy, Default)]
pub struct SingleActivityFiltering {
    /// Allowed deviation of the estimated parameter from `0.5`.
    /// The default configuration uses `0.2`.
    pub config: FilteringConfig,
}
impl BaseCase for SingleActivityFiltering {
    fn base_case(&self, log: &IndexedEventLog) -> Option<Node> {
        let alphabet = log.alphabet();
        if alphabet.len() != 1 {
            return None;
        }
        let traces = log.traces.len() as f64;
        let events = log
            .traces
            .iter()
            .map(|t| t.events.len() as f64)
            .sum::<f64>();
        let p = traces / (events + traces);
        ((p - 0.5).abs() <= self.config.deviation_threshold())
            .then(|| Node::new_leaf(Some(log.resolve(alphabet[0]).clone())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Activity;
    use crate::test_support::indexed_log as log;
    #[test]
    fn accepts_exact_single_activity_and_estimated_boundary() {
        let case = SingleActivityFiltering {
            config: FilteringConfig::new(0.0).unwrap(),
        };
        assert_eq!(
            case.base_case(&log(&[&["a"], &["a"]])),
            Some(Node::new_leaf(Some(Activity::from("a"))))
        );

        let input = log(&[&["a", "a", "a"], &["a", "a", "a"]]);
        assert!(
            SingleActivityFiltering {
                config: FilteringConfig::new(0.25).unwrap()
            }
            .base_case(&input)
            .is_some()
        );
        assert!(
            SingleActivityFiltering {
                config: FilteringConfig::new(0.24).unwrap()
            }
            .base_case(&input)
            .is_none()
        );
    }
    #[test]
    fn accounts_for_empty_traces_and_rejects_other_alphabets() {
        let case = SingleActivityFiltering::default();
        assert!(case.base_case(&log(&[&[], &["a"]])).is_some());
        assert!(case.base_case(&log(&[])).is_none());
        assert!(case.base_case(&log(&[&[], &[]])).is_none());
        assert!(case.base_case(&log(&[&["a"], &["b"]])).is_none());
    }
}
