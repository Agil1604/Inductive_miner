use crate::framework::BaseCase;
use crate::{IndexedEventLog, Node};

///
/// Returns an activity leaf when every trace contains exactly one event of
/// the same activity.
///
#[derive(Debug, Clone, Copy, Default)]
pub struct SingleActivity;

impl BaseCase for SingleActivity {
    fn base_case(&self, log: &IndexedEventLog) -> Option<Node> {
        let activity = &log.traces.first()?.events.first()?.activity;
        log.traces
            .iter()
            .all(|trace| trace.events.len() == 1 && trace.events[0].activity == *activity)
            .then(|| Node::new_leaf(Some(log.resolve(*activity).clone())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Activity;
    use crate::test_support::indexed_log as log;

    #[test]
    fn matches_single_and_duplicate_single_event_traces() {
        for input in [log(&[&["A"]]), log(&[&["A"], &["A"]])] {
            assert_eq!(
                SingleActivity.base_case(&input),
                Some(Node::new_leaf(Some(Activity::from("A"))))
            );
        }
    }

    #[test]
    fn rejects_empty_logs_and_empty_traces() {
        for input in [
            log(&[]),
            log(&[&[]]),
            log(&[&[], &["A"]]),
            log(&[&["A"], &[]]),
        ] {
            assert_eq!(SingleActivity.base_case(&input), None);
        }
    }

    #[test]
    fn rejects_different_activities_and_repeated_executions() {
        for input in [
            log(&[&["A"], &["B"]]),
            log(&[&["A", "A"]]),
            log(&[&["A"], &["A", "A"]]),
        ] {
            assert_eq!(SingleActivity.base_case(&input), None);
        }
    }
}
