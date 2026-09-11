use super::*;
use crate::components::cut_detections::{CutContext, CutStrategy, SequenceCut};
use crate::framework::{DetectCut, SplitLog};
use crate::test_support::log;
use crate::{EventLog, Miner, Node, OperatorType};

fn fitness(input: &EventLog) {
    let tree = IMa::default().mine(input).unwrap();
    let max = input
        .traces
        .iter()
        .map(|t| t.events.len())
        .max()
        .unwrap_or(0);
    let language = crate::test_fitness::language(tree.root(), max);
    for t in &input.traces {
        let word: Vec<_> = t.events.iter().map(|e| e.activity.0.clone()).collect();
        assert!(language.contains(&word), "missing {word:?} in {tree:?}");
    }
}

#[test]
fn interleaved_blocks_and_inclusive_choice_are_discovered() {
    for (input, expected) in [
        (
            log(&[&["a", "b", "c", "d"], &["c", "d", "a", "b"]]),
            OperatorType::Interleaved,
        ),
        (
            log(&[
                &["a", "b"],
                &["c"],
                &["a", "b", "c"],
                &["a", "c", "b"],
                &["c", "a", "b"],
            ]),
            OperatorType::InclusiveChoice,
        ),
    ] {
        assert_eq!(
            ImaCutFinder::default()
                .detect_cut(&input.indexed())
                .unwrap()
                .operator,
            expected
        );
        let tree = IMa::default().mine(&input).unwrap();
        assert!(matches!(tree.root(),Node::Operator(o) if o.operator_type==expected));
        fitness(&input);
    }
}

#[test]
fn interleaving_rejects_return_to_an_earlier_child_l109() {
    let input = log(&[&["a", "b", "c", "a"], &["b", "c", "a"], &["a", "b", "c"]]);
    let indexed = input.indexed();
    assert!(
        InterleavedCut
            .detect(&CutContext::new(&indexed).unwrap())
            .is_none()
    );
    fitness(&input);
}

#[test]
fn strict_sequence_merges_optional_pivot_scope() {
    let input = log(&[&["a"], &["a", "b", "c"]]);
    let indexed = input.indexed();
    let context = CutContext::new(&indexed).unwrap();
    assert_eq!(SequenceCut.detect(&context).unwrap().partitions.len(), 3);
    let strict = SequenceCutStrict.detect(&context).unwrap();
    assert_eq!(strict.partitions.len(), 2);
    assert_eq!(strict.partitions[1].len(), 2);
    fitness(&input);
}

#[test]
fn thesis_l105_distinguishes_coo_hierarchy() {
    let input = log(&[
        &["a", "d"],
        &["a", "b", "d"],
        &["a", "b", "c", "d"],
        &["a", "e"],
        &["a", "b", "e"],
        &["a", "b", "c", "e"],
        &["a", "d", "e"],
        &["a", "b", "d", "e"],
        &["a", "b", "c", "d", "e"],
        &["d", "a"],
        &["b", "a", "d"],
        &["c", "a", "d", "b"],
        &["e", "a"],
        &["e", "b", "a"],
        &["e", "a", "c", "b"],
        &["d", "c", "b", "a"],
        &["a", "e", "d"],
        &["e", "c", "b", "a"],
    ]);
    let cut = ImaCutFinder::default()
        .detect_cut(&input.indexed())
        .unwrap();
    assert_eq!(cut.operator, OperatorType::Concurrent);
    assert_eq!(
        cut.partitions.iter().map(|p| p.len()).collect::<Vec<_>>(),
        vec![3, 2]
    );
    fitness(&input);
}

#[test]
fn new_splitters_preserve_metadata_and_interner() {
    let mut named = log(&[&["a"], &["b"], &["a", "b"], &["b", "a"]]);
    named.traces[2].events[0].lifecycle = crate::Lifecycle::Complete;
    named.traces[2].events[0].timestamp = Some(
        chrono::DateTime::parse_from_rfc3339("2026-10-04T12:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
    );
    let input = named.indexed();
    let cut = CooCut.detect(&CutContext::new(&input).unwrap()).unwrap();
    let split = ImaLogSplitter::default().split_log(&input, &cut);
    assert_eq!(
        split.iter().map(|l| l.traces.len()).collect::<Vec<_>>(),
        vec![3, 3]
    );
    for sublog in &split {
        assert!(input.shares_interner(sublog));
        for trace in &sublog.traces {
            let original = input
                .traces
                .iter()
                .find(|t| t.case_id == trace.case_id)
                .unwrap();
            for event in &trace.events {
                assert!(original.events.contains(event));
            }
        }
    }
    let mut interleaved = cut.clone();
    interleaved.operator = OperatorType::Interleaved;
    let split = ImaLogSplitter::default().split_log(&input, &interleaved);
    assert!(split.iter().all(|l| l.traces.len() == 4));
}

#[test]
fn preserves_all_pairs_of_short_ternary_traces() {
    let mut words: Vec<Vec<&str>> = vec![vec![]];
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
    for a in &words {
        for b in &words {
            fitness(&log(&[a, b]));
        }
    }
}

#[test]
fn preserves_all_pairs_of_short_binary_traces() {
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
    for a in &words {
        for b in &words {
            fitness(&log(&[a, b]));
        }
    }
    fitness(&EventLog::default());
}
