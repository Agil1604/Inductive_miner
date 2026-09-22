use super::*;
use crate::components::fall_throughs::FallThroughStrategy;
use crate::framework::{FallThrough, FallThroughContext};
use crate::{Activity, Event, EventLog, IndexedEventLog, Lifecycle, Node, OperatorType, Trace};
use Lifecycle::{Complete as C, Start as S};
fn log(events: &[(&str, Lifecycle)]) -> IndexedEventLog {
    EventLog {
        traces: vec![Trace {
            case_id: "case".into(),
            events: events
                .iter()
                .map(|(a, l)| Event {
                    activity: Activity::from(*a),
                    lifecycle: *l,
                    timestamp: None,
                })
                .collect(),
        }],
    }
    .indexed()
}
#[test]
fn tau_loops_only_split_closed_executions() {
    let cases = [
        (vec![("a", S), ("a", C), ("a", S), ("a", C)], true),
        (
            vec![("a", S), ("b", S), ("b", C), ("b", S), ("b", C), ("a", C)],
            false,
        ),
    ];
    for (events, applies) in cases {
        let input = log(&events);
        let recurse = |sub: &IndexedEventLog| {
            assert!(input.shares_interner(sub));
            assert_eq!(sub.traces.len(), 2);
            assert!(
                sub.traces
                    .iter()
                    .all(|t| t.events.len() == 2 && t.case_id == "case")
            );
            Ok(Node::new_leaf(None))
        };
        let find_cut = |_: &IndexedEventLog| None;
        let c = FallThroughContext {
            log: &input,
            recurse: &recurse,
            find_cut: &find_cut,
        };
        for strategy in [
            &NonAtomicTauLoop as &dyn FallThroughStrategy,
            &StrictNonAtomicTauLoop,
        ] {
            assert_eq!(strategy.try_apply(&c).unwrap().is_some(), applies);
        }
    }
}
#[test]
fn once_per_trace_extracts_one_instance_but_rejects_repetition() {
    for (events, applies) in [
        (vec![("a", S), ("b", S), ("b", C), ("a", C)], true),
        (vec![("a", S), ("a", C), ("a", S), ("a", C)], false),
    ] {
        let input = log(&events);
        let recurse = |sub: &IndexedEventLog| {
            assert!(input.shares_interner(sub));
            assert_eq!(sub.traces[0].events.len(), 2);
            Ok(Node::new_leaf(None))
        };
        let find_cut = |_: &IndexedEventLog| None;
        let c = FallThroughContext {
            log: &input,
            recurse: &recurse,
            find_cut: &find_cut,
        };
        assert_eq!(
            NonAtomicActivityOncePerTrace
                .try_apply(&c)
                .unwrap()
                .is_some(),
            applies
        );
    }
}
#[test]
fn concurrent_flower_covers_self_overlap_and_empty_input() {
    let input = log(&[("a", S), ("b", S), ("a", S), ("b", C), ("a", C), ("a", C)]);
    let recurse = |_: &IndexedEventLog| panic!("fallback must not recurse");
    let find_cut = |_: &IndexedEventLog| None;
    let c = FallThroughContext {
        log: &input,
        recurse: &recurse,
        find_cut: &find_cut,
    };
    let node = ConcurrentFlowerModel.fall_through(&c).unwrap();
    let Node::Operator(op) = node else {
        panic!("expected concurrent operator")
    };
    assert_eq!(op.operator_type, OperatorType::Concurrent);
    assert_eq!(op.children.len(), 3);
    let expected = ["a", "a", "b"];
    for (child, a) in op.children.iter().zip(expected) {
        assert_eq!(
            *child,
            Node::new_operator_with_children(
                OperatorType::Loop,
                vec![
                    Node::new_leaf(None),
                    Node::new_leaf(Some(Activity::from(a)))
                ]
            )
        );
    }
    let empty = log(&[]);
    let c = FallThroughContext {
        log: &empty,
        recurse: &recurse,
        find_cut: &find_cut,
    };
    assert_eq!(
        ConcurrentFlowerModel.fall_through(&c).unwrap(),
        Node::new_leaf(None)
    );
}
