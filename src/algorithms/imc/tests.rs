use super::IMc;
use crate::test_support::log;
use crate::{EventLog, Miner, Node};

#[test]
fn empty_logs_and_all_empty_traces_produce_valid_silent_trees() {
    for input in [EventLog::default(), log(&[&[]]), log(&[&[], &[]])] {
        let tree = IMc::default().mine(&input).unwrap();
        tree.root().validate().unwrap();
        assert!(crate::test_fitness::language(tree.root(), 0).contains(&Vec::new()));
    }
    assert_eq!(
        IMc::default().mine(&EventLog::default()).unwrap().root(),
        &Node::new_leaf(None)
    );
}

#[test]
fn mines_all_pairs_of_binary_traces_up_to_length_three() {
    let mut words = vec![Vec::new()];
    for len in 1..=3 {
        for bits in 0..(1 << len) {
            words.push(
                (0..len)
                    .map(|i| if bits & (1 << i) == 0 { "a" } else { "b" })
                    .collect::<Vec<_>>(),
            );
        }
    }
    let miner = IMc::default();
    for a in &words {
        for b in &words {
            miner
                .mine(&log(&[a, b]))
                .unwrap()
                .root()
                .validate()
                .unwrap();
        }
    }
}
