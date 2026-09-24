use super::*;
use crate::algorithms::imlc::ImlcCutFinder;
use crate::components::log_splitting::repair_consistency;
use crate::framework::{BaseCase, DetectCut};
use crate::{Activity, Event, EventLog, Lifecycle, Miner, Node, Trace};
use Lifecycle::{Complete as C, Start as S};
fn log(events: &[(&str, Lifecycle)]) -> EventLog {
    EventLog {
        traces: vec![Trace {
            case_id: "case".into(),
            events: events
                .iter()
                .map(|(a, l)| Event {
                    activity: Activity::from(*a),
                    lifecycle: *l,
                    timestamp: Some("2026-01-01T00:00:00Z".parse().unwrap()),
                })
                .collect(),
        }],
    }
}
#[test]
fn repairs_unmatched_starts_without_reordering_original_events() {
    let input = log(&[("a", S), ("a", S), ("a", C), ("b", S)]).indexed();
    let repaired = repair_consistency(&input);
    assert!(input.shares_interner(&repaired));
    let events = &repaired.traces[0].events;
    assert_eq!(events.len(), 6);
    assert_eq!(
        events.iter().map(|e| e.lifecycle).collect::<Vec<_>>(),
        vec![S, C, S, C, S, C]
    );
    assert_eq!(events[1].timestamp, events[0].timestamp);
    assert_eq!(repaired.traces[0].case_id, "case");
    assert_eq!(repair_consistency(&repaired).traces[0].events, *events);
}
#[test]
fn base_case_counts_executions_instead_of_lifecycle_events() {
    let input = log(&[("a", S), ("a", C)]).indexed();
    assert!(
        SingleNonAtomicActivityFiltering {
            config: FilteringConfig::new(0.0).unwrap()
        }
        .base_case(&input)
        .is_some()
    );
    let input = log(&[("a", S), ("a", C), ("a", S), ("a", C)]).indexed();
    assert!(
        SingleNonAtomicActivityFiltering {
            config: FilteringConfig::new(0.0).unwrap()
        }
        .base_case(&input)
        .is_none()
    );
    assert!(
        SingleNonAtomicActivityFiltering::default()
            .base_case(&input)
            .is_some()
    );
}
#[test]
fn first_pass_takes_precedence_and_miner_handles_lifecycle_examples() {
    for input in [
        log(&[("a", S), ("b", S), ("b", C), ("a", C)]),
        log(&[("a", S), ("a", C), ("b", S), ("b", C)]),
        log(&[("a", S), ("b", S), ("a", S), ("b", C), ("a", C), ("a", C)]),
    ] {
        let indexed = input.indexed();
        let expected = ImlcCutFinder::default().detect_cut(&indexed);
        let config = FilteringConfig::new(1.0).unwrap();
        let finder = ImflcCutFinder {
            config,
            ..Default::default()
        };
        if let Some(expected) = expected {
            assert_eq!(finder.detect_cut(&indexed), Some(expected));
        }
        config.imflc().mine(&input).unwrap();
        IMflc::default().mine(&input).unwrap();
    }
    assert_eq!(
        IMflc::default().mine(&EventLog::default()).unwrap().root(),
        &Node::new_leaf(None)
    );
}

#[test]
fn filtered_pass_keeps_original_overlap_evidence() {
    use crate::IndexedCut;
    use crate::components::cut_detections::{NonAtomicCutContext, NonAtomicCutStrategy};
    struct Reject;
    impl NonAtomicCutStrategy for Reject {
        fn detect(&self, _: &NonAtomicCutContext<'_>) -> Option<IndexedCut> {
            None
        }
    }
    struct Check;
    impl NonAtomicCutStrategy for Check {
        fn detect(&self, c: &NonAtomicCutContext<'_>) -> Option<IndexedCut> {
            assert_eq!(c.dfg.edges.len(), 2);
            assert_eq!(c.concurrency.edges.len(), 1);
            assert_eq!(c.dfg.activities.len(), 3);
            None
        }
    }
    let frequent = log(&[("a", S), ("b", S), ("b", C), ("a", C), ("c", S), ("c", C)]);
    let rare = log(&[("a", S), ("a", C), ("b", S), ("b", C)]);
    let mut input = frequent.clone();
    input
        .traces
        .extend(frequent.traces.iter().cycle().take(9).cloned());
    input.traces.extend(rare.traces);
    let finder = NonAtomicFilteringCutFinder {
        unfiltered: Reject,
        filtered: Check,
        config: FilteringConfig::new(0.2).unwrap(),
    };
    assert!(finder.detect_cut(&input.indexed()).is_none());
}

#[test]
fn filtered_sequence_split_repairs_removed_completions() {
    use crate::framework::SplitLog;
    use crate::{Cut, OperatorType};
    use std::collections::HashSet;
    let input = log(&[("a", S), ("b", S), ("a", C), ("b", C)]).indexed();
    let a = input.traces[0].events[0].activity;
    let b = input.traces[0].events[1].activity;
    let cut = Cut {
        operator: OperatorType::Sequence,
        partitions: vec![HashSet::from([a]), HashSet::from([b])],
    };
    let children = ImflcLogSplitter::default().split_log(&input, &cut);
    for child in children {
        assert!(input.shares_interner(&child));
        for trace in &child.traces {
            let mut active = 0usize;
            for event in &trace.events {
                if event.lifecycle == S {
                    active += 1;
                }
                if event.lifecycle == C && active > 0 {
                    active -= 1;
                }
            }
            assert_eq!(active, 0);
        }
    }
}
