use super::*;

use crate::algorithms::im::ImCutFinder;

#[test]
fn standard_im_cuts_take_precedence_including_nonbinary_cuts() {
    for input in [
        indexed_log(&[&["a"], &["b"], &["c"]]),
        indexed_log(&[&["a", "b", "c"]]),
        indexed_log(&[&["a", "b"], &["b", "a"]]),
        indexed_log(&[&["a"], &["a", "b", "a"], &["a", "c", "a"]]),
    ] {
        let standard = ImCutFinder::default().detect_cut(&input).unwrap();
        assert_eq!(ImcCutFinder.detect_cut(&input), Some(standard));
    }
    let input = indexed_log(&[&["a", "b", "c"]]);
    assert_eq!(ImcCutFinder.detect_cut(&input).unwrap().partitions.len(), 3);
    assert_eq!(
        ImcCutFinder
            .find_scored_cut(&input)
            .unwrap()
            .cut
            .partitions
            .len(),
        2
    );
}
use crate::test_support::{indexed_log, log};
use crate::{Activity, IMc, Miner};

#[test]
fn table_6_2_all_nine_evidence_rows() {
    let rows = [
        ((false, false, false, false), 0, vec![1, 2, 3, 4, 5, 6]),
        ((false, false, true, false), 1, vec![3, 4, 5, 6]),
        ((false, false, false, true), 2, vec![3, 4, 5, 6]),
        ((false, false, true, true), 3, vec![4, 5, 6]),
        ((true, false, true, false), 1, vec![4, 6]),
        ((true, false, true, true), 4, vec![6]),
        ((false, true, false, true), 2, vec![5, 6]),
        ((false, true, true, true), 5, vec![6]),
        ((true, true, true, true), 6, vec![]),
    ];
    for ((ab, ba, eab, eba), observed, stronger) in rows {
        let p = probabilities::estimate(ab, ba, eab, eba, 4.0).0;
        assert!((p.iter().sum::<f64>() - 1.0).abs() < 1e-12);
        for (i, &value) in p.iter().enumerate() {
            let expected = if i == observed {
                if stronger.is_empty() { 1.0 } else { 0.8 }
            } else if stronger.contains(&i) {
                0.2 / stronger.len() as f64
            } else {
                0.0
            };
            assert!((value - expected).abs() < 1e-12);
        }
    }
}

#[test]
fn eventually_follows_uses_trace_order_not_graph_reachability() {
    let input = indexed_log(&[&["a", "c", "b"], &["b", "c"], &["c", "a"]]);
    let r = ActivityRelations::from_log(&input);
    let p = r.pair(0, 1).0;
    assert!(p[1] >= 0.5);
    assert_eq!(p[2], 0.0);
    assert_eq!(p[0], 0.0);
}

#[test]
fn guards_binary_partition_validation_and_orientation() {
    for input in [
        indexed_log(&[]),
        indexed_log(&[&["a"]]),
        indexed_log(&[&[], &["a", "b"]]),
    ] {
        assert!(ImcCutFinder.detect_cut(&input).is_none());
    }
    let input = indexed_log(&[&["b", "a"]]);
    let result = ImcCutFinder.find_scored_cut(&input).unwrap();
    assert_eq!(result.cut.operator, OperatorType::Sequence);
    assert_eq!(
        input.resolve(*result.cut.partitions[0].iter().next().unwrap()),
        &Activity::from("b")
    );
    result.cut.validate(&input.activities()).unwrap();
    let r = ActivityRelations::from_log(&input);
    assert!(score_partition(&r, &[0], &[0], OperatorType::Xor).is_none());
    assert!(score_partition(&r, &[0], &[2], OperatorType::Sequence).is_none());
    assert!(score_partition(&r, &[0], &[1], OperatorType::Concurrent).is_none());
    assert!(score_partition(&r, &[0], &[1], OperatorType::Loop).is_none());
}

#[test]
fn thesis_l99_recovers_missing_edge_loop_and_score() {
    let input = indexed_log(&[
        &["a", "b", "c", "d", "e", "f", "a", "c", "b", "d"],
        &["a", "c", "d", "b", "e", "f", "a", "b", "c", "d"],
        &["c", "a", "b", "d"],
        &["c", "a", "d", "b"],
        &["c", "d", "a", "b"],
    ]);
    let result = ImcCutFinder.find_scored_cut(&input).unwrap();
    assert_eq!(result.cut.operator, OperatorType::Loop);
    assert!(ImCutFinder::default().detect_cut(&input).is_none());
    assert_eq!(ImcCutFinder.detect_cut(&input), Some(result.cut.clone()));
    let names: std::collections::HashSet<_> = result.cut.partitions[0]
        .iter()
        .map(|&a| input.resolve(a).0.as_str())
        .collect();
    assert_eq!(names, ["a", "b", "c", "d"].into_iter().collect());

    assert!((result.probability - 191.0 / 264.0).abs() < 1e-12);
    let r = ActivityRelations::from_log(&input);

    let mut oracle: f64 = 0.0;
    for starts in 1..4 {
        for ends in 1..4 {
            let mut sum = 0.0;
            for a in 0..4 {
                for b in 4..6 {
                    let forward = [1, 3].contains(&a) && starts & (1 << (b - 4)) != 0;
                    let reverse = [0, 2].contains(&a) && ends & (1 << (b - 4)) != 0;
                    let p = r.pair(a, b).0;
                    if forward {
                        sum += p[4];
                    }
                    if reverse {
                        sum += p[5];
                    }
                    if !forward && !reverse {
                        sum += p[3];
                    }
                }
            }
            oracle = oracle.max(sum / 8.0);
        }
    }
    assert!((oracle - result.probability).abs() < 1e-12);
    assert!(
        (score_partition(&r, &[0, 1, 2, 3], &[4, 5], OperatorType::Loop).unwrap()
            - result.probability)
            .abs()
            < 1e-12
    );
}

#[test]
fn composed_miner_handles_thesis_l100_and_empty_traces() {
    let input = log(&[
        &["c", "d", "e", "f", "d", "e", "f", "d", "e"],
        &["b", "a", "d", "e"],
        &["a", "b", "d", "e", "f", "d", "e"],
        &["c", "g"],
    ]);
    let cut = ImcCutFinder.detect_cut(&input.indexed()).unwrap();
    assert_eq!(cut.operator, OperatorType::Sequence);
    let tree = IMc::default().mine(&input).unwrap();
    tree.root().validate().unwrap();
    let optional = IMc::default().mine(&log(&[&[], &["a"]])).unwrap();
    assert!(
        matches!(optional.root(), crate::Node::Operator(o) if o.operator_type == OperatorType::Xor)
    );
}

#[test]
fn search_is_deterministic_and_always_returns_valid_cuts() {
    for words in [
        vec![&["a"][..], &["b"], &["c"]],
        vec![&["a", "b"][..], &["b", "a"]],
    ] {
        let input = indexed_log(&words);
        let first = ImcCutFinder.detect_cut(&input).unwrap();
        first.validate(&input.activities()).unwrap();
        for _ in 0..5 {
            assert_eq!(ImcCutFinder.detect_cut(&input).unwrap(), first);
        }
    }
}
