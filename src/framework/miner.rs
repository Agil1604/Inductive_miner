use super::{BaseCase, DetectCut, FallThrough, SplitLog};
use crate::{EventLog, IndexedEventLog, Node, ProcessTree};

#[derive(thiserror::Error, Debug)]
pub enum MinerError {
    #[error(transparent)]
    Tree(#[from] crate::TreeError),
    #[error(transparent)]
    Cut(#[from] crate::CutError),
    #[error("splitter must return one sublog per cut part, in partition order")]
    InvalidSplitCount,
    #[error("sublog contains activities outside its corresponding cut part")]
    InvalidSplitAlphabet,
    #[error("splitter returned a sublog with a different activity interner")]
    DifferentInterner,
}

///
/// A trait for miners to implement the recursive mining algorithm.
///
pub trait Miner {
    fn mine(&self, log: &EventLog) -> Result<ProcessTree, MinerError>;
}

///
/// An inductive miner.
///
pub struct InductiveMiner<B, C, S, F> {
    base_case: B,
    cut_finder: C,
    log_splitter: S,
    fall_through: F,
}

impl<B, C, S, F> InductiveMiner<B, C, S, F>
where
    B: BaseCase,
    C: DetectCut,
    S: SplitLog,
    F: FallThrough,
{
    ///
    /// Creates a new inductive miner with the given plugins.
    ///
    pub fn new(base_case: B, cut_finder: C, log_splitter: S, fall_through: F) -> Self {
        Self {
            base_case,
            cut_finder,
            log_splitter,
            fall_through,
        }
    }

    ///
    /// Recursively mines a node from the given event log.
    ///
    pub fn mine_node(&self, log: &IndexedEventLog) -> Result<Node, MinerError> {
        if let Some(tree) = self.base_case.base_case(log) {
            tree.validate()?;
            return Ok(tree);
        }

        if let Some(cut) = self.cut_finder.detect_cut(log) {
            cut.validate(&log.activities())?;
            let sublogs = self.log_splitter.split_log(log, &cut);
            if sublogs.len() != cut.partitions.len() {
                return Err(MinerError::InvalidSplitCount);
            }

            for (sublog, part) in sublogs.iter().zip(&cut.partitions) {
                if !log.shares_interner(sublog) {
                    return Err(MinerError::DifferentInterner);
                }
                if !sublog.activities().is_subset(part) {
                    return Err(MinerError::InvalidSplitAlphabet);
                }
            }
            let children = sublogs
                .iter()
                .map(|l| self.mine_node(l))
                .collect::<Result<Vec<_>, _>>()?;

            return Ok(Node::new_operator_with_children(cut.operator, children));
        }

        let node = self.fall_through.fall_through(log);
        node.validate()?;
        Ok(node)
    }
}

impl<B: BaseCase, C: DetectCut, S: SplitLog, F: FallThrough> Miner for InductiveMiner<B, C, S, F> {
    fn mine(&self, log: &EventLog) -> Result<ProcessTree, MinerError> {
        Ok(ProcessTree::new(self.mine_node(&log.indexed())?)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Activity, IndexedCut, OperatorType};
    struct Plugins;
    impl BaseCase for Plugins {
        fn base_case(&self, _: &IndexedEventLog) -> Option<Node> {
            None
        }
    }
    impl DetectCut for Plugins {
        fn detect_cut(&self, log: &IndexedEventLog) -> Option<IndexedCut> {
            Some(IndexedCut {
                operator: OperatorType::Sequence,
                partitions: log
                    .activities()
                    .into_iter()
                    .map(|a| std::collections::HashSet::from([a]))
                    .collect(),
            })
        }
    }
    struct BadSplit(bool);
    impl SplitLog for BadSplit {
        fn split_log(&self, log: &IndexedEventLog, _: &IndexedCut) -> Vec<IndexedEventLog> {
            if self.0 {
                vec![log.clone(), log.clone()]
            } else {
                vec![]
            }
        }
    }
    impl FallThrough for Plugins {
        fn fall_through(&self, _: &IndexedEventLog) -> Node {
            Node::new_leaf(None)
        }
    }
    #[test]
    fn rejects_bad_splits_before_recursing() {
        let log = EventLog {
            traces: vec![crate::Trace {
                case_id: String::new(),
                events: ["a", "b"]
                    .into_iter()
                    .map(|a| crate::Event {
                        activity: Activity(a.into()),
                        lifecycle: crate::Lifecycle::Unknown,
                        timestamp: None,
                    })
                    .collect(),
            }],
        };
        assert!(matches!(
            InductiveMiner::new(Plugins, Plugins, BadSplit(false), Plugins).mine(&log),
            Err(MinerError::InvalidSplitCount)
        ));
        assert!(matches!(
            InductiveMiner::new(Plugins, Plugins, BadSplit(true), Plugins).mine(&log),
            Err(MinerError::InvalidSplitAlphabet)
        ));
    }
    #[test]
    fn rejects_sublogs_with_an_independent_interner() {
        struct ForeignSplit;
        impl SplitLog for ForeignSplit {
            fn split_log(&self, _: &IndexedEventLog, _: &IndexedCut) -> Vec<IndexedEventLog> {
                vec![
                    crate::test_support::log(&[&["a"]]).indexed(),
                    crate::test_support::log(&[&["b"]]).indexed(),
                ]
            }
        }
        let log = crate::test_support::log(&[&["a", "b"]]);
        assert!(matches!(
            InductiveMiner::new(Plugins, Plugins, ForeignSplit, Plugins).mine(&log),
            Err(MinerError::DifferentInterner)
        ));
    }
}
