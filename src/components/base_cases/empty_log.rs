use crate::framework::BaseCase;
use crate::{IndexedEventLog, Node};

///
/// Returns a silent leaf when the log contains no traces.
///
#[derive(Debug, Clone, Copy, Default)]
pub struct EmptyLog;

impl BaseCase for EmptyLog {
    fn base_case(&self, log: &IndexedEventLog) -> Option<Node> {
        log.traces.is_empty().then(|| Node::new_leaf(None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Activity, Event, EventLog, Lifecycle, Trace};

    #[test]
    fn returns_tau_for_a_log_without_traces() {
        assert_eq!(
            EmptyLog.base_case(&EventLog::default().indexed()),
            Some(Node::new_leaf(None))
        );
    }

    #[test]
    fn does_not_match_a_log_containing_an_empty_trace() {
        let log = EventLog {
            traces: vec![Trace::default()],
        };
        assert_eq!(EmptyLog.base_case(&log.indexed()), None);
    }

    #[test]
    fn does_not_match_a_log_containing_events() {
        let log = EventLog {
            traces: vec![Trace {
                case_id: "case-1".into(),
                events: vec![Event {
                    activity: Activity::from("A"),
                    lifecycle: Lifecycle::Unknown,
                    timestamp: None,
                }],
            }],
        };
        assert_eq!(EmptyLog.base_case(&log.indexed()), None);
    }
}
