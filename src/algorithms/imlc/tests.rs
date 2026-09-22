use super::*;
use crate::framework::{DetectCut, SplitLog};
use crate::{Activity, Event, EventLog, Lifecycle, Trace};
use Lifecycle::{Complete as C, Start as S};

#[test]
fn lifecycle_splitting_retains_events_metadata_and_consistency() {
    let cases = [
        vec![vec![("a", S), ("a", C)], vec![("b", S), ("b", C)]],
        vec![vec![("a", S), ("a", C), ("b", S), ("b", C)]],
        vec![vec![("a", S), ("b", S), ("b", C), ("a", C)]],
        vec![vec![
            ("a", S),
            ("a", C),
            ("b", S),
            ("b", C),
            ("a", S),
            ("a", C),
        ]],
    ];
    for traces in cases {
        let log = EventLog {
            traces: traces
                .iter()
                .enumerate()
                .map(|(i, events)| Trace {
                    case_id: format!("case-{i}"),
                    events: events
                        .iter()
                        .map(|(a, lifecycle)| Event {
                            activity: Activity::from(*a),
                            lifecycle: *lifecycle,
                            timestamp: Some("2026-01-01T00:00:00Z".parse().unwrap()),
                        })
                        .collect(),
                })
                .collect(),
        }
        .indexed();
        let cut = ImlcCutFinder::default().detect_cut(&log).unwrap();
        check_split(&log, &cut);
    }
}

fn check_split(log: &crate::IndexedEventLog, cut: &crate::IndexedCut) {
    let children = ImlcLogSplitter::default().split_log(log, cut);
    assert_eq!(children.len(), cut.partitions.len());
    assert_eq!(
        children
            .iter()
            .flat_map(|l| &l.traces)
            .map(|t| t.events.len())
            .sum::<usize>(),
        log.traces.iter().map(|t| t.events.len()).sum::<usize>()
    );
    for (child, part) in children.iter().zip(&cut.partitions) {
        assert!(log.shares_interner(child));
        for trace in &child.traces {
            let original = log
                .traces
                .iter()
                .find(|t| t.case_id == trace.case_id)
                .unwrap();
            let mut active = std::collections::HashMap::new();
            for event in &trace.events {
                assert!(part.contains(&event.activity));
                assert!(original.events.contains(event));
                let count = active.entry(event.activity).or_insert(0usize);
                match event.lifecycle {
                    S => *count += 1,
                    C => {
                        assert!(*count > 0);
                        *count -= 1;
                    }
                    _ => unreachable!(),
                }
            }
            assert!(active.values().all(|n| *n == 0));
        }
    }
}

#[test]
fn interleaved_splitting_preserves_complete_execution_blocks() {
    let mut input = EventLog {
        traces: vec![Trace {
            case_id: "case".into(),
            events: [("a", S), ("a", C), ("b", S), ("b", C)]
                .iter()
                .map(|(a, l)| Event {
                    activity: Activity::from(*a),
                    lifecycle: *l,
                    timestamp: None,
                })
                .collect(),
        }],
    };
    let mut reverse = input.traces[0].clone();
    reverse.case_id = "reverse-case".into();
    reverse.events.rotate_left(2);
    input.traces.push(reverse);
    let log = input.indexed();
    use crate::components::cut_detections::{NonAtomicCutContext, NonAtomicCutStrategy};
    let cut = NonAtomicInterleavedCut
        .detect(&NonAtomicCutContext::new(&log).unwrap())
        .unwrap();
    check_split(&log, &cut);
}
