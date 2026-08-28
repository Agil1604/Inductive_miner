use super::Activity;
use std::collections::HashSet;

///
/// Process tree leaf types
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeafType {
    /// Non-silent activity
    Activity(Activity),
    /// Silent activity
    Tau,
}

///
/// A leaf of the process tree
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Leaf {
    /// Leaf activity of [`LeafType`]
    pub activity_label: LeafType,
}

impl Leaf {
    ///
    /// Create an instance of [`Leaf`]
    ///
    pub fn new(label: Option<Activity>) -> Self {
        if let Some(activity) = label {
            Self {
                activity_label: LeafType::Activity(activity),
            }
        } else {
            Self {
                activity_label: LeafType::Tau,
            }
        }
    }
}

///
/// Process tree operator types
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorType {
    /// Sequence
    Sequence,
    /// Exclusive choice
    Xor,
    /// Concurrency
    Concurrent,
    /// First child is the body; remaining children are alternative redo branches.
    Loop,
    /// Interleaved
    Interleaved,
    /// Inclusive choice
    InclusiveChoice,
}

///
/// An operator of the process tree
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Operator {
    /// Type of an operator ([`OperatorType`])
    pub operator_type: OperatorType,
    /// The children of the operator
    pub children: Vec<Node>,
}

impl Operator {
    ///
    /// Create an instance of [`Operator`] with no children
    ///
    pub fn new(operator_type: OperatorType) -> Self {
        Self {
            operator_type,
            children: Vec::new(),
        }
    }
}

#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
pub enum TreeError {
    #[error("cannot add child to a leaf node")]
    LeafCannotHaveChildren,
    #[error("{operator:?} requires at least {minimum} children, got {actual}")]
    InvalidArity {
        operator: OperatorType,
        minimum: usize,
        actual: usize,
    },
}

///
/// Node of the process tree
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    /// Operator node
    Operator(Operator),
    /// Leaf node
    Leaf(Leaf),
}

impl Node {
    ///
    /// Create an operator node with no children
    ///
    pub fn new_operator(op_type: OperatorType) -> Self {
        Self::Operator(Operator::new(op_type))
    }

    ///
    /// Create an operator node with children vector
    ///
    pub fn new_operator_with_children(op: OperatorType, children: Vec<Node>) -> Self {
        Self::Operator(Operator {
            operator_type: op,
            children,
        })
    }

    ///
    /// Create a leaf node
    ///
    pub fn new_leaf(leaf_label: Option<Activity>) -> Self {
        Self::Leaf(Leaf::new(leaf_label))
    }

    ///
    /// Add one child node to an existing operator node
    ///
    pub fn add_child(&mut self, child: Node) -> Result<(), TreeError> {
        match self {
            Node::Operator(op) => {
                op.children.push(child);
                Ok(())
            }
            Node::Leaf(_) => Err(TreeError::LeafCannotHaveChildren),
        }
    }

    ///
    /// Validate the process tree node recursively
    ///
    pub fn validate(&self) -> Result<(), TreeError> {
        if let Self::Operator(op) = self {
            let minimum = if op.operator_type == OperatorType::Loop {
                2
            } else {
                1
            };
            if op.children.len() < minimum {
                return Err(TreeError::InvalidArity {
                    operator: op.operator_type,
                    minimum,
                    actual: op.children.len(),
                });
            }
            for child in &op.children {
                child.validate()?;
            }
        }
        Ok(())
    }
}

///
/// Process tree
///
#[derive(Debug)]
pub struct ProcessTree {
    root: Node,
}

impl ProcessTree {
    ///
    /// Create an instance of [`ProcessTree`]
    ///
    pub fn new(root: Node) -> Result<Self, TreeError> {
        root.validate()?;
        Ok(Self { root })
    }

    ///
    /// Getter for root node of the process tree
    ///
    pub fn root(&self) -> &Node {
        &self.root
    }
}

///
/// Cut of a process tree
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cut<A: Eq + std::hash::Hash = Activity> {
    pub operator: OperatorType,
    pub partitions: Vec<HashSet<A>>,
}

#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
pub enum CutError {
    #[error("a nontrivial cut requires at least two nonempty parts")]
    EmptyPartition,
    #[error("cut parts overlap")]
    Overlap,
    #[error("cut parts must cover exactly the log activity alphabet")]
    AlphabetMismatch,
}

impl<A: Clone + Eq + std::hash::Hash> Cut<A> {
    ///
    /// Validate the cut against the given activity alphabet
    ///
    pub fn validate(&self, alphabet: &HashSet<A>) -> Result<(), CutError> {
        if self.partitions.len() < 2 || self.partitions.iter().any(HashSet::is_empty) {
            return Err(CutError::EmptyPartition);
        }
        let mut union = HashSet::new();
        for part in &self.partitions {
            for activity in part {
                if !union.insert(activity.clone()) {
                    return Err(CutError::Overlap);
                }
            }
        }
        if &union != alphabet {
            return Err(CutError::AlphabetMismatch);
        }
        Ok(())
    }
}

#[cfg(test)]
mod validation_tests {
    use super::*;
    #[test]
    fn rejects_invalid_arity_recursively() {
        assert!(ProcessTree::new(Node::new_operator(OperatorType::Xor)).is_err());
        let leaf = Node::new_leaf(None);
        assert!(
            ProcessTree::new(Node::new_operator_with_children(
                OperatorType::Loop,
                vec![leaf.clone()]
            ))
            .is_err()
        );
        assert!(
            ProcessTree::new(Node::new_operator_with_children(
                OperatorType::Loop,
                vec![leaf.clone(), leaf]
            ))
            .is_ok()
        );
        assert!(
            ProcessTree::new(Node::new_operator_with_children(
                OperatorType::Sequence,
                vec![Node::new_operator(OperatorType::Xor)]
            ))
            .is_err()
        );
    }
    #[test]
    fn checks_partition_coverage_overlap_and_empty_parts() {
        let a = HashSet::from([Activity("a".into())]);
        let b = HashSet::from([Activity("b".into())]);
        let alphabet = a.union(&b).cloned().collect();
        let mut cut = Cut {
            operator: OperatorType::Sequence,
            partitions: vec![a.clone(), b],
        };
        assert!(cut.validate(&alphabet).is_ok());
        cut.partitions[1] = a;
        assert_eq!(cut.validate(&alphabet), Err(CutError::Overlap));
        cut.partitions[1] = HashSet::from([Activity("c".into())]);
        assert_eq!(cut.validate(&alphabet), Err(CutError::AlphabetMismatch));
        cut.partitions[1].clear();
        assert_eq!(cut.validate(&alphabet), Err(CutError::EmptyPartition));
    }
}
