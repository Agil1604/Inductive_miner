use crate::framework::BaseCase;
use crate::{IndexedEventLog, Lifecycle, Node};

/// Returns an activity leaf when every trace contains one execution of the same
/// activity.
///
/// An execution is a start followed by a completion of the same activity.
/// A lone completion is also accepted; a lone unknown event is
/// treated as atomic, matching this library's lifecycle graph builders.
/// Enqueue and other transitions are ignored. Empty logs, traces without an
/// execution, repeated executions, and unmatched starts do not match.
#[derive(Debug, Clone, Copy, Default)]
pub struct SingleNonAtomicActivity;

impl BaseCase for SingleNonAtomicActivity {
    fn base_case(&self, log: &IndexedEventLog) -> Option<Node> {
        let mut activity = None;
        for trace in &log.traces {
            let mut events = trace.events.iter().filter(|event| {
                matches!(
                    event.lifecycle,
                    Lifecycle::Start | Lifecycle::Complete | Lifecycle::Unknown
                )
            });
            let first = events.next()?;
            match first.lifecycle {
                Lifecycle::Start => {
                    let complete = events.next()?;
                    if complete.lifecycle != Lifecycle::Complete
                        || complete.activity != first.activity
                    {
                        return None;
                    }
                }
                Lifecycle::Complete | Lifecycle::Unknown => {}
                _ => unreachable!(),
            }
            if events.next().is_some() || activity.is_some_and(|a| a != first.activity) {
                return None;
            }
            activity = Some(first.activity);
        }
        activity.map(|a| Node::new_leaf(Some(log.resolve(a).clone())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::base_cases::{BaseCaseChain, EmptyLog, WithoutEmptyTraces};
    use crate::{Activity, Event, EventLog, Trace};
    use Lifecycle::{Complete as C, Start as S, Unknown as U};

    fn log(traces: &[&[(&str, Lifecycle)]]) -> IndexedEventLog {
        EventLog {
            traces: traces
                .iter()
                .map(|events| Trace {
                    case_id: String::new(),
                    events: events
                        .iter()
                        .map(|(name, lifecycle)| Event {
                            activity: Activity::from(*name),
                            lifecycle: *lifecycle,
                            timestamp: None,
                        })
                        .collect(),
                })
                .collect(),
        }
        .indexed()
    }

    #[test]
    fn accepts_single_instances_and_mixed_atomic_traces() {
        let input = log(&[
            &[("a", S), ("a", C)],
            &[("a", C)],
            &[("a", U)],
            &[
                ("ignored", Lifecycle::Enqueue),
                ("a", S),
                ("ignored", Lifecycle::Other),
                ("a", C),
            ],
        ]);
        assert_eq!(
            SingleNonAtomicActivity.base_case(&input),
            Some(Node::new_leaf(Some(Activity::from("a"))))
        );
    }

    #[test]
    fn rejects_empty_and_inconsistent_traces_and_multiple_instances() {
        for events in [
            vec![],
            vec![("a", S)],
            vec![("a", C), ("a", S)],
            vec![("a", S), ("b", C)],
            vec![("a", S), ("a", C), ("a", S), ("a", C)],
            vec![("a", S), ("a", S), ("a", C), ("a", C)],
            vec![("a", C), ("a", C)],
            vec![("a", Lifecycle::Other)],
        ] {
            assert_eq!(SingleNonAtomicActivity.base_case(&log(&[&events])), None);
        }
        assert_eq!(SingleNonAtomicActivity.base_case(&log(&[])), None);
        assert_eq!(
            SingleNonAtomicActivity.base_case(&log(&[&[("a", S), ("a", C)], &[("b", C)]])),
            None
        );
        assert_eq!(
            SingleNonAtomicActivity.base_case(&log(&[&[("a", C)], &[]])),
            None
        );
    }

    #[test]
    fn imlc_composition_handles_empty_log_and_defers_empty_traces() {
        let base = WithoutEmptyTraces(BaseCaseChain {
            first: EmptyLog,
            second: SingleNonAtomicActivity,
        });
        assert_eq!(base.base_case(&log(&[])), Some(Node::new_leaf(None)));
        assert_eq!(base.base_case(&log(&[&[]])), None);
        assert_eq!(
            base.base_case(&log(&[&[("a", S), ("a", C)]])),
            Some(Node::new_leaf(Some(Activity::from("a"))))
        );
    }
}
