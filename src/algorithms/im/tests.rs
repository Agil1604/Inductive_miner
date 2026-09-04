use super::{ImCutFinder, ImFallThrough, ImLogSplitter};
use crate::framework::{DetectCut, FallThrough, FallThroughContext, SplitLog};
use crate::test_support::log;
use crate::{Activity, EventLog, IM, LeafType, Miner, Node, OperatorType};
#[path = "../../../tests/support/im_fitness.rs"]
mod fitness;
use crate as library;
use fitness::check_fitness;
use fitness::language;
fn root_op(log: &EventLog) -> OperatorType {
    match IM::default().mine(log).unwrap().root() {
        Node::Operator(o) => o.operator_type,
        _ => panic!("expected operator"),
    }
}
#[test]
fn detects_all_four_cuts() {
    for (input, expected) in [
        (log(&[&["a"], &["b"]]), OperatorType::Xor),
        (log(&[&["a", "b", "c"]]), OperatorType::Sequence),
        (log(&[&["a", "b"], &["b", "a"]]), OperatorType::Concurrent),
        (
            log(&[&["a"], &["a", "b", "a"], &["a", "c", "a"]]),
            OperatorType::Loop,
        ),
    ] {
        assert_eq!(
            ImCutFinder::default()
                .detect_cut(&input.indexed())
                .unwrap()
                .operator,
            expected
        );
        assert_eq!(root_op(&input), expected);
    }
}
#[test]
fn base_cases_and_empty_traces() {
    assert!(
        matches!(IM::default().mine(&EventLog::default()).unwrap().root(),Node::Leaf(l) if l.activity_label == LeafType::Tau)
    );
    assert!(
        matches!(IM::default().mine(&log(&[&["a"], &["a"]])).unwrap().root(),Node::Leaf(l) if l.activity_label == LeafType::Activity(Activity("a".into())))
    );
    assert_eq!(root_op(&log(&[&[], &["a"]])), OperatorType::Xor);
    assert_eq!(root_op(&log(&[&["a", "a"], &["a"]])), OperatorType::Loop);
    IM::default()
        .mine(&log(&[&[], &[]]))
        .unwrap()
        .root()
        .validate()
        .unwrap();
}
#[test]
fn minimum_distance_prevents_false_parallel_cut() {
    let l = log(&[&["a", "b", "a"]]);
    assert_eq!(
        ImCutFinder::default()
            .detect_cut(&l.indexed())
            .unwrap()
            .operator,
        OperatorType::Loop
    );

    let l = log(&[&["a", "b", "a"], &["a", "a"], &["b", "a", "b"], &["b", "b"]]);
    assert_eq!(
        ImCutFinder::default()
            .detect_cut(&l.indexed())
            .unwrap()
            .operator,
        OperatorType::Concurrent
    );
}
#[test]
fn splitting_preserves_metadata_multiplicity_and_empty_projections() {
    let l = log(&[&["a", "b"], &["b"], &["a", "b"]]);
    let c = ImCutFinder::default().detect_cut(&l.indexed()).unwrap();
    let split = ImLogSplitter::default().split_log(&l.indexed(), &c);
    assert_eq!(split.len(), 2);
    assert_eq!(split[0].traces.len(), 3);
    assert!(split[0].traces[1].events.is_empty());
    assert_eq!(split[1].traces[0].case_id, "0");
    let l = log(&[&["a", "b", "a", "c", "a"]]);
    let c = ImCutFinder::default().detect_cut(&l.indexed()).unwrap();
    let split = ImLogSplitter::default().split_log(&l.indexed(), &c);
    assert_eq!(split[0].traces.len(), 3);
    assert_eq!(split[1].traces.len(), 1);
    assert_eq!(split[2].traces.len(), 1);
}

#[test]
fn fitness_for_all_pairs_of_binary_traces_up_to_length_three() {
    let mut words = vec![vec![]];
    for len in 1..=3 {
        for bits in 0..(1 << len) {
            words.push(
                (0..len)
                    .map(|i| if bits & (1 << i) == 0 { "a" } else { "b" })
                    .collect::<Vec<_>>(),
            );
        }
    }
    for x in &words {
        for y in &words {
            check_fitness(&log(&[x, y]));
        }
    }
}
#[test]
fn thesis_examples_preserve_fitness() {
    check_fitness(&log(&[
        &["a", "b", "c", "d"],
        &["d", "a", "b"],
        &["a", "d", "c"],
        &["b", "c", "d"],
    ]));
    check_fitness(&log(&[&["a", "c", "d"], &["b", "c", "e"]]));
}
#[test]
fn fallthroughs_preserve_observed_behavior() {
    for input in [
        log(&[&[], &["a"]]),
        log(&[
            &["a", "b", "c", "d"],
            &["d", "a", "b"],
            &["a", "d", "c"],
            &["b", "c", "d"],
        ]),
        log(&[&["a", "b", "a"], &["b", "a", "b"]]),
    ] {
        let indexed = input.indexed();
        let recurse = |log: &crate::IndexedEventLog| IM::default().mine_node(log);
        let finder = ImCutFinder::default();
        let find_cut = |log: &crate::IndexedEventLog| finder.detect_cut(log);
        let context = FallThroughContext {
            log: &indexed,
            recurse: &recurse,
            find_cut: &find_cut,
        };
        let node = ImFallThrough::default().fall_through(&context).unwrap();
        node.validate().unwrap();
        let max = input.traces.iter().map(|t| t.events.len()).max().unwrap();
        let accepted = language(&node, max);
        for t in input.traces {
            assert!(
                accepted.contains(
                    &t.events
                        .into_iter()
                        .map(|e| e.activity.0)
                        .collect::<Vec<_>>()
                )
            );
        }
    }
}

#[test]
fn fitness_for_all_pairs_of_ternary_traces_up_to_length_three() {
    let mut words = vec![vec![]];
    for len in 1..=3 {
        for mut code in 0..3usize.pow(len) {
            let mut word = Vec::new();
            for _ in 0..len {
                word.push(["a", "b", "c"][code % 3]);
                code /= 3;
            }
            words.push(word);
        }
    }
    for x in &words {
        for y in &words {
            check_fitness(&log(&[x, y]));
        }
    }
}
