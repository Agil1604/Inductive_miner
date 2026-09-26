use super::*;
use crate::algorithms::imlc::ImlcCutFinder;
use crate::framework::DetectCut;
use crate::{Activity, Event, EventLog, Lifecycle, Miner, OperatorType, Trace};
use Lifecycle::{Complete as C, Start as S};
fn log(traces: &[&[(&str, Lifecycle)]]) -> EventLog {
    EventLog {
        traces: traces
            .iter()
            .map(|t| Trace {
                case_id: "case".into(),
                events: t
                    .iter()
                    .map(|(a, l)| Event {
                        activity: Activity::from(*a),
                        lifecycle: *l,
                        timestamp: None,
                    })
                    .collect(),
            })
            .collect(),
    }
}
#[test]
fn imlc_cuts_take_precedence_and_preserve_nonbinary_partitions() {
    let input = log(&[
        &[("a", S), ("a", C)],
        &[("b", S), ("b", C)],
        &[("c", S), ("c", C)],
    ])
    .indexed();
    let expected = ImlcCutFinder::default().detect_cut(&input).unwrap();
    assert_eq!(expected.partitions.len(), 3);
    assert_eq!(ImclcCutFinder.detect_cut(&input), Some(expected));
}
#[test]
fn lifecycle_solver_recognizes_overlap_and_atomic_sequence() {
    let overlap = log(&[&[("a", S), ("b", S), ("b", C), ("a", C)]]).indexed();
    let scored = ImclcCutFinder.find_scored_cut(&overlap).unwrap();
    assert_eq!(scored.cut.operator, OperatorType::Concurrent);
    assert_eq!(scored.probability, 1.0);
    let atomic = log(&[&[("a", C), ("b", C)]]).indexed();
    let paired = log(&[&[("a", S), ("a", C), ("b", S), ("b", C)]]).indexed();
    let a = ImclcCutFinder.find_scored_cut(&atomic).unwrap();
    let b = ImclcCutFinder.find_scored_cut(&paired).unwrap();
    assert_eq!(a.cut, b.cut);
    assert_eq!(a.probability, b.probability);
}
#[test]
fn mines_empty_optional_and_self_overlapping_logs() {
    for input in [
        EventLog::default(),
        log(&[&[]]),
        log(&[&[], &[("a", S), ("a", C)]]),
        log(&[&[("a", S), ("a", S), ("a", C), ("a", C)]]),
    ] {
        IMclc::default().mine(&input).unwrap();
    }
}

#[test]
fn probabilistic_fallback_handles_log_without_an_imlc_cut() {
    let events: Vec<_> = ["a", "a", "b", "a", "b", "b"]
        .into_iter()
        .flat_map(|a| [(a, S), (a, C)])
        .collect();
    let input = log(&[&events]);
    let indexed = input.indexed();
    assert!(ImlcCutFinder::default().detect_cut(&indexed).is_none());
    let cut = ImclcCutFinder.detect_cut(&indexed).unwrap();
    assert_eq!(cut.partitions.len(), 2);
    IMclc::default().mine(&input).unwrap();
}
