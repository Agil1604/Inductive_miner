use super::*;
use crate::test_support::indexed_log as log;
use crate::{Activity, Lifecycle, Trace};

fn cut(log: &IndexedEventLog, operator: OperatorType, parts: &[&[&str]]) -> IndexedCut {
    IndexedCut {
        operator,
        partitions: parts
            .iter()
            .map(|p| {
                p.iter()
                    .map(|name| log.interner().id(&Activity::from(*name)).unwrap())
                    .collect()
            })
            .collect(),
    }
}
fn sequences(log: &IndexedEventLog) -> Vec<Vec<&str>> {
    log.traces
        .iter()
        .map(|t| {
            t.events
                .iter()
                .map(|e| log.resolve(e.activity).0.as_str())
                .collect()
        })
        .collect()
}
#[test]
fn xor_assigns_whole_traces_preserving_duplicates() {
    let input = log(&[&["a", "b"], &["c"], &["a", "b"]]);
    let c = cut(&input, OperatorType::Xor, &[&["c"], &["a", "b"]]);
    let split = XorSplit.split_log(&input, &c);
    assert_eq!(sequences(&split[0]), vec![vec!["c"]]);
    assert_eq!(sequences(&split[1]), vec![vec!["a", "b"], vec!["a", "b"]]);
    assert_eq!(split[1].traces[1].case_id, "2");
    assert!(split.iter().all(|s| input.shares_interner(s)));
}
#[test]
fn sequence_preserves_partition_order_and_empty_projections() {
    let input = log(&[&["b", "a"], &["a"], &["b", "a"]]);
    let c = cut(&input, OperatorType::Sequence, &[&["b"], &["a"]]);
    let split = SequenceSplit.split_log(&input, &c);
    assert_eq!(sequences(&split[0]), vec![vec!["b"], vec![], vec!["b"]]);
    assert_eq!(sequences(&split[1]), vec![vec!["a"], vec!["a"], vec!["a"]]);
    assert!(split.iter().all(|s| input.shares_interner(s)));
}
#[test]
fn concurrency_preserves_event_order_and_metadata() {
    let input = log(&[&["a", "c", "b"], &["c", "a"], &[]]);
    let mut traces = input.traces.clone();
    let timestamp = chrono::DateTime::parse_from_rfc3339("2026-10-03T10:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    traces[0].events[2].timestamp = Some(timestamp);
    traces[0].events[2].lifecycle = Lifecycle::Complete;
    let input = input.with_traces(traces);
    let c = cut(&input, OperatorType::Concurrent, &[&["a", "b"], &["c"]]);
    let split = ConcurrentSplit.split_log(&input, &c);
    assert_eq!(
        sequences(&split[0]),
        vec![vec!["a", "b"], vec!["a"], vec![]]
    );
    assert_eq!(sequences(&split[1]), vec![vec!["c"], vec!["c"], vec![]]);
    assert_eq!(split[0].traces[0].events[1].timestamp, Some(timestamp));
    assert_eq!(split[0].traces[0].events[1].lifecycle, Lifecycle::Complete);
    assert_eq!(split[0].traces[1].case_id, "1");
    assert!(split.iter().all(|s| input.shares_interner(s)));
}
#[test]
fn loop_splits_maximal_runs_with_body_first_and_multiple_redos() {
    let input = log(&[&["a", "b", "c", "c", "a", "b", "d", "a", "b"], &["a", "b"]]);
    let c = cut(&input, OperatorType::Loop, &[&["a", "b"], &["c"], &["d"]]);
    let split = LoopSplit.split_log(&input, &c);
    assert_eq!(sequences(&split[0]), vec![vec!["a", "b"]; 4]);
    assert_eq!(sequences(&split[1]), vec![vec!["c", "c"]]);
    assert_eq!(sequences(&split[2]), vec![vec!["d"]]);
    assert_eq!(
        split[0]
            .traces
            .iter()
            .map(|t| t.case_id.as_str())
            .collect::<Vec<_>>(),
        vec!["0", "0", "0", "1"]
    );
    assert!(split.iter().all(|s| input.shares_interner(s)));
}
#[test]
fn dispatcher_uses_the_matching_splitter_and_rejects_unsupported_operators() {
    let input = log(&[&["a"], &["b"]]);
    let dispatcher = LogSplitter {
        xor: XorSplit,
        sequence: SequenceSplit,
        concurrent: ConcurrentSplit,
        loop_split: LoopSplit,
    };
    let c = cut(&input, OperatorType::Xor, &[&["a"], &["b"]]);
    assert_eq!(dispatcher.split_log(&input, &c).len(), 2);
    for splitter in [
        &SequenceSplit as &dyn SplitLog,
        &ConcurrentSplit,
        &LoopSplit,
    ] {
        assert!(splitter.split_log(&input, &c).is_empty());
    }
    let unsupported = cut(&input, OperatorType::InclusiveChoice, &[&["a"], &["b"]]);
    assert!(dispatcher.split_log(&input, &unsupported).is_empty());
}
#[test]
fn empty_sublogs_retain_the_original_interner() {
    let input = log(&[&["a"], &["b"]]);
    let empty = input.with_traces(vec![Trace::default()]);
    let c = cut(&input, OperatorType::Concurrent, &[&["a"], &["b"]]);
    let split = ConcurrentSplit.split_log(&empty, &c);
    assert!(
        split.iter().all(|s| input.shares_interner(s)
            && s.traces.len() == 1
            && s.traces[0].events.is_empty())
    );
}

#[test]
fn filtered_xor_selects_majority_and_keeps_duplicates() {
    let input = log(&[
        &["a", "b"],
        &["c", "c", "c"],
        &["a", "b", "c"],
        &["a", "c"],
        &[],
    ]);
    let c = cut(&input, OperatorType::Xor, &[&["a", "b"], &["c"]]);
    let split = XorSplitFiltering.split_log(&input, &c);
    assert_eq!(
        sequences(&split[0]),
        vec![vec!["a", "b"], vec!["a", "b"], vec!["a"], vec![]]
    );
    assert_eq!(sequences(&split[1]), vec![vec!["c", "c", "c"]]);
    assert!(split.iter().all(|s| input.shares_interner(s)));
    let input = log(&[&["a", "b", "b"]]);
    let c = cut(&input, OperatorType::Xor, &[&["a"], &["b"]]);
    let split = XorSplitFiltering.split_log(&input, &c);
    assert!(split[0].traces.is_empty());
    assert_eq!(sequences(&split[1]), vec![vec!["b", "b"]]);
}
#[test]
fn filtered_sequence_uses_per_trace_boundaries_and_discards_wrong_side_events() {
    let input = log(&[&["a", "b", "a", "a", "b"], &["b", "b", "a"], &["a", "b"]]);
    let c = cut(&input, OperatorType::Sequence, &[&["a"], &["b"]]);
    let split = SequenceSplitFiltering.split_log(&input, &c);
    assert_eq!(
        sequences(&split[0]),
        vec![vec!["a", "a", "a"], vec![], vec!["a"]]
    );
    assert_eq!(
        sequences(&split[1]),
        vec![vec!["b"], vec!["b", "b"], vec!["b"]]
    );
    assert!(split.iter().all(|s| input.shares_interner(s)));
    let input = log(&[&["a", "b", "c"], &["b", "a", "c"], &["c", "a", "b", "c"]]);
    let c = cut(&input, OperatorType::Sequence, &[&["a", "b"], &["c"]]);
    let split = SequenceSplitFiltering.split_log(&input, &c);
    assert_eq!(
        sequences(&split[0]),
        vec![vec!["a", "b"], vec!["b", "a"], vec!["a", "b"]]
    );
    assert_eq!(sequences(&split[1]), vec![vec!["c"]; 3]);
}
#[test]
fn filtered_loop_repairs_missing_initial_and_final_body_runs() {
    let input = log(&[&["b", "a"], &["a", "b"], &["b"], &[], &["a", "b", "a"]]);
    let c = cut(&input, OperatorType::Loop, &[&["a"], &["b"]]);
    let split = LoopSplitFiltering.split_log(&input, &c);
    assert_eq!(
        sequences(&split[0]),
        vec![
            vec![],
            vec!["a"],
            vec!["a"],
            vec![],
            vec![],
            vec![],
            vec![],
            vec!["a"],
            vec!["a"]
        ]
    );
    assert_eq!(sequences(&split[1]), vec![vec!["b"]; 4]);
    assert!(split.iter().all(|s| input.shares_interner(s)));
}
#[test]
fn filtered_splitters_equal_standard_splits_for_valid_cuts() {
    for (input, operator, parts, standard, filtered) in [
        (
            log(&[&["a"], &["b"], &["a"]]),
            OperatorType::Xor,
            vec![vec!["a"], vec!["b"]],
            &XorSplit as &dyn SplitLog,
            &XorSplitFiltering as &dyn SplitLog,
        ),
        (
            log(&[&["a", "b"], &["b"]]),
            OperatorType::Sequence,
            vec![vec!["a"], vec!["b"]],
            &SequenceSplit,
            &SequenceSplitFiltering,
        ),
        (
            log(&[&["a", "b", "a"], &["a"]]),
            OperatorType::Loop,
            vec![vec!["a"], vec!["b"]],
            &LoopSplit,
            &LoopSplitFiltering,
        ),
    ] {
        let refs: Vec<_> = parts.iter().map(|p| p.as_slice()).collect();
        let c = cut(&input, operator, &refs);
        let left = standard.split_log(&input, &c);
        let right = filtered.split_log(&input, &c);
        for (a, b) in left.iter().zip(&right) {
            assert_eq!(sequences(a), sequences(b));
            assert_eq!(
                a.traces.iter().map(|t| &t.case_id).collect::<Vec<_>>(),
                b.traces.iter().map(|t| &t.case_id).collect::<Vec<_>>()
            );
        }
    }
}
#[test]
fn imf_dispatcher_preserves_metadata_on_retained_events() {
    let input = log(&[&["a", "b", "a", "a", "b"]]);
    let mut traces = input.traces.clone();
    let timestamp = chrono::DateTime::parse_from_rfc3339("2026-10-03T12:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    traces[0].events[3].timestamp = Some(timestamp);
    traces[0].events[3].lifecycle = Lifecycle::Complete;
    let input = input.with_traces(traces);
    let c = cut(&input, OperatorType::Sequence, &[&["a"], &["b"]]);
    let split = crate::FilteringConfig::default()
        .imf_log_splitter()
        .split_log(&input, &c);
    assert_eq!(split[0].traces[0].events[2].timestamp, Some(timestamp));
    assert_eq!(split[0].traces[0].events[2].lifecycle, Lifecycle::Complete);
    assert_eq!(split[0].traces[0].case_id, "0");
}
