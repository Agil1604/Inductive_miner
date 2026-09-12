use super::*;
use crate::components::fall_throughs::FallThroughStrategy;
use crate::framework::{BaseCase, DetectCut, FallThroughContext};
use crate::test_fitness as fitness;
use crate::test_support::log;
use crate::{Activity, EventLog, Miner, MinerError, Node, OperatorType};

fn leaf(a: &str) -> Node {
    Node::new_leaf(Some(Activity::from(a)))
}

#[test]
fn defaults_and_empty_logs() {
    for input in [EventLog::default(), log(&[&[]]), log(&[&[], &[]])] {
        assert_eq!(
            IMf::default().mine(&input).unwrap().root(),
            &Node::new_leaf(None)
        );
    }
    let input = log(&[&["a", "b"]]);
    assert_eq!(
        IMf::default().mine(&input).unwrap().root(),
        FilteringConfig::default()
            .imf()
            .mine(&input)
            .unwrap()
            .root()
    );
}

#[test]
fn empty_trace_threshold_is_inclusive_and_precedes_base_cases() {
    let input = log(&[&[], &["a"], &["a"], &["a"], &["a"]]);
    let indexed = input.indexed();
    let config = FilteringConfig::new(0.2).unwrap();
    assert!(config.base_cases().base_case(&indexed).is_none());
    assert!(config.imf_cut_finder().detect_cut(&indexed).is_none());
    assert_eq!(
        config.imf().mine(&input).unwrap().root(),
        &Node::new_operator_with_children(OperatorType::Xor, vec![Node::new_leaf(None), leaf("a")])
    );
    assert_eq!(
        FilteringConfig::new(0.21)
            .unwrap()
            .imf()
            .mine(&input)
            .unwrap()
            .root(),
        &leaf("a")
    );
    assert_eq!(
        FilteringConfig::new(1.0)
            .unwrap()
            .imf()
            .mine(&input)
            .unwrap()
            .root(),
        &leaf("a")
    );
}

#[test]
fn empty_trace_filtering_shares_interner_and_propagates_errors() {
    let input = log(&[&[], &["a"]]).indexed();
    let recurse = |sublog: &crate::IndexedEventLog| {
        assert!(input.shares_interner(sublog));
        assert_eq!(sublog.traces.len(), 1);
        assert_eq!(sublog.traces[0].case_id, "1");
        Err(MinerError::InvalidSplitCount)
    };
    let find_cut = |_: &crate::IndexedEventLog| None;
    let context = FallThroughContext {
        log: &input,
        recurse: &recurse,
        find_cut: &find_cut,
    };
    assert!(matches!(
        EmptyTracesFiltering::default().try_apply(&context),
        Err(MinerError::InvalidSplitCount)
    ));
}

#[test]
fn filters_rare_reverse_behavior_and_recursively_filters_empty_projections() {
    let mut words = vec![&["a", "b"][..]; 10];
    words.extend([&["b", "a"][..], &["a", "b", "a"][..]]);
    let input = log(&words);
    let tree = FilteringConfig::new(0.3)
        .unwrap()
        .imf()
        .mine(&input)
        .unwrap();
    assert_eq!(
        tree.root(),
        &Node::new_operator_with_children(OperatorType::Sequence, vec![leaf("a"), leaf("b")])
    );
    assert!(!fitness::language(tree.root(), 3).contains(&vec!["b".into(), "a".into()]));
}

#[test]
fn configured_threshold_reaches_recursive_single_activity_base_case() {
    let input = log(&[&["a", "a", "b"]]);
    let filtered = FilteringConfig::new(0.2)
        .unwrap()
        .imf()
        .mine(&input)
        .unwrap();
    assert_eq!(
        filtered.root(),
        &Node::new_operator_with_children(OperatorType::Sequence, vec![leaf("a"), leaf("b")])
    );
    let unfiltered = FilteringConfig::new(0.0)
        .unwrap()
        .imf()
        .mine(&input)
        .unwrap();
    assert!(fitness::language(unfiltered.root(), 3).contains(&vec![
        "a".into(),
        "a".into(),
        "b".into()
    ]));
}

#[test]
fn zero_threshold_preserves_all_pairs_of_short_binary_traces() {
    let mut words: Vec<Vec<&str>> = vec![vec![]];
    for len in 1..=3 {
        for bits in 0..(1 << len) {
            words.push(
                (0..len)
                    .map(|i| if bits & (1 << i) == 0 { "a" } else { "b" })
                    .collect(),
            );
        }
    }
    let miner = FilteringConfig::new(0.0).unwrap().imf();
    for a in &words {
        for b in &words {
            let input = log(&[a, b]);
            let tree = miner.mine(&input).unwrap();
            let language = fitness::language(tree.root(), 3);
            for word in [a, b] {
                assert!(
                    language.contains(&word.iter().map(|s| s.to_string()).collect::<Vec<_>>()),
                    "missing {word:?} in {tree:?}"
                );
            }
        }
    }
    fitness::check_fitness(&log(&[&["a", "b"], &["b", "a"]]));
}
