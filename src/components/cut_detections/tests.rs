use super::*;
use crate::test_support::indexed_log as log;
use crate::{Activity, OperatorType};

#[test]
fn independent_detectors_return_expected_partitions() {
    for (detector, input, operator, expected) in [
        (
            &CutFinder(XorCut) as &dyn DetectCut,
            log(&[&["a"], &["b"]]),
            OperatorType::Xor,
            vec![vec!["a"], vec!["b"]],
        ),
        (
            &CutFinder(SequenceCut),
            log(&[&["b", "b", "a"]]),
            OperatorType::Sequence,
            vec![vec!["b"], vec!["a"]],
        ),
        (
            &CutFinder(ConcurrentCut),
            log(&[&["a", "b"], &["b", "a"]]),
            OperatorType::Concurrent,
            vec![vec!["a"], vec!["b"]],
        ),
        (
            &CutFinder(LoopCut),
            log(&[&["a"], &["a", "b", "a"], &["a", "c", "a"]]),
            OperatorType::Loop,
            vec![vec!["a"], vec!["b"], vec!["c"]],
        ),
    ] {
        let cut = detector.detect_cut(&input).unwrap();
        assert_eq!(cut.operator, operator);
        assert_eq!(
            cut.partitions,
            expected
                .into_iter()
                .map(|p| p
                    .into_iter()
                    .map(|name| input.interner().id(&Activity::from(name)).unwrap())
                    .collect())
                .collect::<Vec<_>>()
        );
        cut.validate(&input.activities()).unwrap();
    }
}
#[test]
fn detectors_reject_empty_traces_and_trivial_alphabets() {
    for detector in [
        &CutFinder(XorCut) as &dyn DetectCut,
        &CutFinder(SequenceCut),
        &CutFinder(ConcurrentCut),
        &CutFinder(LoopCut),
    ] {
        for input in [
            log(&[]),
            log(&[&[]]),
            log(&[&["a"]]),
            log(&[&[], &["a", "b"]]),
        ] {
            assert!(detector.detect_cut(&input).is_none());
        }
    }
}
#[test]
fn concurrency_respects_minimum_self_distance() {
    assert!(
        CutFinder(ConcurrentCut)
            .detect_cut(&log(&[&["a", "b", "a"]]))
            .is_none()
    );
}
#[test]
fn sequence_merges_mutually_unreachable_activities() {
    let input = log(&[&["a", "c", "d"], &["b", "c", "e"]]);
    let cut = CutFinder(SequenceCut).detect_cut(&input).unwrap();
    assert_eq!(cut.partitions.len(), 3);
    assert_eq!(
        cut.partitions[0],
        [
            input.interner().id(&Activity::from("a")).unwrap(),
            input.interner().id(&Activity::from("b")).unwrap()
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(
        cut.partitions[2],
        [
            input.interner().id(&Activity::from("d")).unwrap(),
            input.interner().id(&Activity::from("e")).unwrap()
        ]
        .into_iter()
        .collect()
    );
}
#[test]
fn chain_returns_first_match_and_falls_back() {
    let chain = CutFinder(CutChain {
        first: XorCut,
        second: SequenceCut,
    });
    assert_eq!(
        chain.detect_cut(&log(&[&["a"], &["b"]])).unwrap().operator,
        OperatorType::Xor
    );
    assert_eq!(
        chain.detect_cut(&log(&[&["a", "b"]])).unwrap().operator,
        OperatorType::Sequence
    );
    assert!(
        chain
            .detect_cut(&log(&[&["a", "b"], &["b", "a"]]))
            .is_none()
    );
}

#[test]
fn chain_shares_one_context_and_witnesses_are_lazy() {
    use std::cell::Cell;
    struct Observe<'a>(&'a Cell<usize>);
    impl CutStrategy for Observe<'_> {
        fn detect(&self, context: &CutContext<'_>) -> Option<IndexedCut> {
            let address = context.dfg() as *const _ as usize;
            if self.0.get() == 0 {
                self.0.set(address);
            } else {
                assert_eq!(self.0.get(), address);
            }
            assert!(context.witnesses.get().is_none());
            None
        }
    }
    let seen = Cell::new(0);
    let finder = CutFinder(CutChain {
        first: Observe(&seen),
        second: Observe(&seen),
    });
    let input = log(&[&["a", "b", "a"]]);
    assert!(finder.detect_cut(&input).is_none());
    assert_ne!(seen.get(), 0);
    let context = CutContext::new(&input).unwrap();
    assert!(context.witnesses.get().is_none());
    assert!(context.witnesses().contains(&(
        input.interner().id(&Activity::from("a")).unwrap(),
        input.interner().id(&Activity::from("b")).unwrap()
    )));
    assert!(std::ptr::eq(context.witnesses(), context.witnesses()));
}
