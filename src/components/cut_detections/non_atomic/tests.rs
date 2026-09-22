use crate::algorithms::imlc::ImlcCutFinder;
use crate::framework::DetectCut;
use crate::{IndexedEventLog, Lifecycle, OperatorType};

use super::*;
use crate::{Activity, Event, EventLog, Trace};
use Lifecycle::{Complete as C, Start as S};
fn log(traces: &[&[(&str, Lifecycle)]]) -> IndexedEventLog {
    EventLog {
        traces: traces
            .iter()
            .map(|t| Trace {
                case_id: String::new(),
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
    .indexed()
}
#[test]
fn nested_overlap_is_concurrent_not_xor_or_sequence() {
    let l = log(&[&[("a", S), ("b", S), ("b", C), ("a", C)]]);
    assert_eq!(
        ImlcCutFinder::default().detect_cut(&l).unwrap().operator,
        OperatorType::Concurrent
    );
    assert!(
        NonAtomicInterleavedCut
            .detect(&NonAtomicCutContext::new(&l).unwrap())
            .is_none()
    );
}
#[test]
fn basic_cuts_and_empty_trace_guard() {
    for (l, op) in [
        (
            log(&[&[("a", S), ("a", C)], &[("b", S), ("b", C)]]),
            OperatorType::Xor,
        ),
        (
            log(&[&[("a", S), ("a", C), ("b", S), ("b", C)]]),
            OperatorType::Sequence,
        ),
    ] {
        let cut = ImlcCutFinder::default().detect_cut(&l).unwrap();
        assert_eq!(cut.operator, op);
        assert_eq!(cut.partitions.len(), 2);
    }
    assert!(
        ImlcCutFinder::default()
            .detect_cut(&log(&[&[], &[("a", C), ("b", C)]]))
            .is_none()
    );
    assert!(
        ImlcCutFinder::default()
            .detect_cut(&log(&[&[("a", C)]]))
            .is_none()
    );
}
#[test]
fn interleaving_and_loop() {
    let l = log(&[
        &[("a", S), ("a", C), ("b", S), ("b", C)],
        &[("b", S), ("b", C), ("a", S), ("a", C)],
    ]);
    let c = NonAtomicCutContext::new(&l).unwrap();
    assert_eq!(
        NonAtomicInterleavedCut.detect(&c).unwrap().operator,
        OperatorType::Interleaved
    );
    let l = log(&[&[("a", S), ("a", C), ("b", S), ("b", C), ("a", S), ("a", C)]]);
    let cut = ImlcCutFinder::default().detect_cut(&l).unwrap();
    assert_eq!(cut.operator, OperatorType::Loop);
    assert!(
        cut.partitions[0]
            .iter()
            .all(|a| l.resolve(*a) == &Activity::from("a"))
    );
}
