use super::*;
use crate::components::cut_detections::{CutContext, CutStrategy, filter_dfg};
use crate::components::log_splitting::InterleavedSplitFiltering;
use crate::framework::{DetectCut, SplitLog};
use crate::test_support::log;
use crate::{Activity, EventLog, IndexedCut, Miner, Node, OperatorType};

#[test]
fn zero_threshold_preserves_short_binary_logs() {
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
    let miner = FilteringConfig::new(0.0).unwrap().imfa();
    for a in &words {
        for b in &words {
            let input = log(&[a, b]);
            let tree = miner.mine(&input).unwrap();
            let accepted = crate::test_fitness::language(tree.root(), 3);
            for t in &input.traces {
                let word: Vec<_> = t.events.iter().map(|e| e.activity.0.clone()).collect();
                assert!(accepted.contains(&word), "missing {word:?} in {tree:?}");
            }
        }
    }
}

fn cut(input: &crate::IndexedEventLog, parts: &[&[&str]]) -> IndexedCut {
    IndexedCut {
        operator: OperatorType::Interleaved,
        partitions: parts
            .iter()
            .map(|p| {
                p.iter()
                    .map(|a| input.interner().id(&Activity::from(*a)).unwrap())
                    .collect()
            })
            .collect(),
    }
}
fn words(input: &crate::IndexedEventLog) -> Vec<Vec<String>> {
    input
        .traces
        .iter()
        .map(|t| {
            t.events
                .iter()
                .map(|e| input.resolve(e.activity).0.clone())
                .collect()
        })
        .collect()
}

#[test]
fn thesis_interleaved_split_example_removes_misplaced_b() {
    let named = log(&[&["a", "a", "b", "a", "a", "b", "b", "b", "b", "c"], &[]]);
    let mut named = named;
    named.traces[0].events[5].lifecycle = crate::Lifecycle::Complete;
    named.traces[0].events[5].timestamp = Some(
        chrono::DateTime::parse_from_rfc3339("2026-10-04T12:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
    );
    let input = named.indexed();
    let split =
        InterleavedSplitFiltering.split_log(&input, &cut(&input, &[&["a"], &["b"], &["c"]]));
    assert_eq!(words(&split[0]), vec![vec!["a"; 4], vec![]]);
    assert_eq!(words(&split[1]), vec![vec!["b"; 4], vec![]]);
    assert_eq!(words(&split[2]), vec![vec!["c"], vec![]]);
    assert_eq!(split[1].traces[0].events[0], input.traces[0].events[5]);
    for sublog in &split {
        assert!(input.shares_interner(sublog));
        assert_eq!(sublog.traces[0].case_id, "0");
    }
}

#[test]
fn ima_cuts_take_precedence() {
    let input = log(&[&["a", "b", "c", "d"], &["c", "d", "a", "b"]]).indexed();
    assert_eq!(
        FilteringConfig::new(1.0)
            .unwrap()
            .imfa_cut_finder()
            .detect_cut(&input),
        crate::algorithms::ima::ImaCutFinder::default().detect_cut(&input)
    );
}

#[test]
fn filtered_interleaved_detection_repairs_rare_block_violation() {
    let mut traces = vec![&["a", "b", "c", "d"][..]; 20];
    traces.extend(vec![&["c", "d", "a", "b"][..]; 20]);
    traces.push(&["a", "c", "b", "d"]);
    let input = log(&traces).indexed();
    let context = CutContext::new(&input).unwrap();
    let filtered = context.with_dfg(filter_dfg(
        context.dfg(),
        FilteringConfig::new(0.2).unwrap(),
    ));
    assert!(InterleavedCut.detect(&filtered).is_none());
    let found = InterleavedCutFiltering.detect(&filtered).unwrap();
    assert_eq!(
        found.partitions.iter().map(|p| p.len()).collect::<Vec<_>>(),
        vec![2, 2]
    );
    let tree = FilteringConfig::default()
        .imfa()
        .mine(&log(&traces))
        .unwrap();
    assert!(matches!(tree.root(),Node::Operator(o) if o.operator_type==OperatorType::Interleaved));
}

#[test]
fn filtering_base_cases_empty_traces_and_default_configuration() {
    for input in [EventLog::default(), log(&[&[]])] {
        IMfa::default().mine(&input).unwrap();
    }
    let input = log(&[&[], &["a"], &["a"], &["a"], &["a"]]);
    let tree = FilteringConfig::new(0.21)
        .unwrap()
        .imfa()
        .mine(&input)
        .unwrap();
    assert_eq!(tree.root(), &Node::new_leaf(Some(Activity::from("a"))));
    let input = log(&[&["a", "a"]]);
    assert_eq!(
        IMfa::default().mine(&input).unwrap().root(),
        &Node::new_leaf(Some(Activity::from("a")))
    );
    assert_eq!(FilteringConfig::default().deviation_threshold(), 0.2);
    assert!(FilteringConfig::new(f64::NAN).is_err());
}
