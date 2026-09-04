use super::*;
use crate::test_support::indexed_log as log;
use crate::{IndexedEventLog, OperatorType};
use std::cell::RefCell;

fn names(log: &IndexedEventLog) -> Vec<Vec<String>> {
    log.traces
        .iter()
        .map(|t| {
            t.events
                .iter()
                .map(|e| log.resolve(e.activity).0.clone())
                .collect()
        })
        .collect()
}
#[test]
fn optional_strategies_use_the_supplied_recursion_and_shared_interner() {
    for (strategy, input, expected, operator) in [
        (
            &EmptyTraces as &dyn FallThroughStrategy,
            log(&[&[], &["a"]]),
            vec![vec![vec!["a"]]],
            OperatorType::Xor,
        ),
        (
            &ActivityOncePerTrace,
            log(&[&["a", "b"], &["b", "a"]]),
            vec![vec![vec!["b"], vec!["b"]]],
            OperatorType::Concurrent,
        ),
        (
            &StrictTauLoop,
            log(&[&["a", "b", "a", "b"]]),
            vec![vec![vec!["a", "b"], vec!["a", "b"]]],
            OperatorType::Loop,
        ),
        (
            &TauLoop,
            log(&[&["a", "c", "a", "b"]]),
            vec![vec![vec!["a", "c"], vec!["a", "b"]]],
            OperatorType::Loop,
        ),
    ] {
        let calls = RefCell::new(Vec::new());
        let recurse = |sublog: &IndexedEventLog| {
            assert!(input.shares_interner(sublog));
            calls.borrow_mut().push(names(sublog));
            Ok(Node::new_leaf(None))
        };
        let find_cut = |_: &IndexedEventLog| None;
        let context = FallThroughContext {
            log: &input,
            recurse: &recurse,
            find_cut: &find_cut,
        };
        let node = strategy.try_apply(&context).unwrap().unwrap();
        assert!(matches!(node,Node::Operator(ref op) if op.operator_type==operator));
        assert_eq!(*calls.borrow(), expected);
    }
}
#[test]
fn activity_concurrent_uses_the_configured_cut_finder() {
    let input = log(&[&["a", "b", "c"]]);
    let calls = RefCell::new(Vec::new());
    let recurse = |sublog: &IndexedEventLog| {
        calls.borrow_mut().push(names(sublog));
        Ok(Node::new_leaf(None))
    };
    let find_cut = |other: &IndexedEventLog| {
        assert!(input.shares_interner(other));
        Some(crate::IndexedCut {
            operator: OperatorType::Sequence,
            partitions: other
                .alphabet()
                .into_iter()
                .map(|a| [a].into_iter().collect())
                .collect(),
        })
    };
    let context = FallThroughContext {
        log: &input,
        recurse: &recurse,
        find_cut: &find_cut,
    };
    assert!(ActivityConcurrent.try_apply(&context).unwrap().is_some());
    assert_eq!(*calls.borrow(), vec![vec![vec!["a"]], vec![vec!["b", "c"]]]);
}
#[test]
fn strategies_return_none_when_inapplicable() {
    let input = log(&[&["a", "a", "b", "b"]]);
    let recurse = |_: &IndexedEventLog| panic!("unexpected recursion");
    let find_cut = |_: &IndexedEventLog| None;
    let context = FallThroughContext {
        log: &input,
        recurse: &recurse,
        find_cut: &find_cut,
    };
    for strategy in [
        &EmptyTraces as &dyn FallThroughStrategy,
        &ActivityOncePerTrace,
        &ActivityConcurrent,
        &StrictTauLoop,
    ] {
        assert!(strategy.try_apply(&context).unwrap().is_none());
    }
    let no_loop = log(&[&["a", "b"]]);
    let context = FallThroughContext {
        log: &no_loop,
        recurse: &recurse,
        find_cut: &find_cut,
    };
    assert!(TauLoop.try_apply(&context).unwrap().is_none());
}
#[test]
fn flower_model_is_a_guaranteed_fallback_without_recursion() {
    for input in [
        log(&[]),
        log(&[&[]]),
        log(&[&["a", "b"]]),
        log(&[&[], &["a"]]),
    ] {
        let recurse = |_: &IndexedEventLog| panic!("flower should not recurse");
        let find_cut = |_: &IndexedEventLog| None;
        let context = FallThroughContext {
            log: &input,
            recurse: &recurse,
            find_cut: &find_cut,
        };
        FlowerModel
            .fall_through(&context)
            .unwrap()
            .validate()
            .unwrap();
    }
}
#[test]
fn chain_short_circuits_and_propagates_recursive_errors() {
    struct Never;
    impl FallThroughStrategy for Never {
        fn try_apply(&self, _: &FallThroughContext<'_>) -> Result<Option<Node>, MinerError> {
            panic!("later strategy called")
        }
    }
    let chain = FallThroughChain {
        first: EmptyTraces,
        second: Never,
    };
    let input = log(&[&[], &["a"]]);
    let find_cut = |_: &IndexedEventLog| None;
    let recurse = |_: &IndexedEventLog| Ok(Node::new_leaf(None));
    let context = FallThroughContext {
        log: &input,
        recurse: &recurse,
        find_cut: &find_cut,
    };
    assert!(chain.try_apply(&context).unwrap().is_some());
    let fail = |_: &IndexedEventLog| Err(MinerError::InvalidSplitCount);
    let context = FallThroughContext {
        log: &input,
        recurse: &fail,
        find_cut: &find_cut,
    };
    assert!(matches!(
        chain.try_apply(&context),
        Err(MinerError::InvalidSplitCount)
    ));
}
