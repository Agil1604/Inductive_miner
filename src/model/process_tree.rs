use super::Activity;
use std::collections::HashSet;

/// Observable or silent behaviour represented by a leaf.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeafType {
    /// Non-silent activity.
    Activity(Activity),
    /// A silent step.
    Tau,
}

/// A terminal process tree node, representing an activity or a silent step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Leaf {
    /// The activity to execute, or [`LeafType::Tau`] for silent behaviour.
    pub activity_label: LeafType,
}

impl Leaf {
    /// Creates an activity leaf for `Some(activity)` or a silent leaf for `None`.
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

/// Ways to combine the behaviour of child nodes.
///
/// Validation requires at least two children for [`Self::Loop`] and at least
/// one for every other operator. Representing an operator here does not imply
/// that every mining algorithm or splitter supports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorType {
    /// Executes all children in their stored order.
    Sequence,
    /// Executes exactly one child, chosen as an alternative.
    Xor,
    /// Executes all children concurrently, allowing their events to interleave.
    Concurrent,
    /// Executes the first child (body), then zero or more repetitions of a
    /// chosen redo child followed by the body. Remaining children are alternative
    /// redo branches; execution starts and ends with the body.
    Loop,
    /// Executes all children in any order without interleaving their executions.
    Interleaved,
    /// Executes a nonempty selection of children, allowing concurrent execution.
    InclusiveChoice,
}

/// An internal process tree node combining a vector of child nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Operator {
    /// How the children are combined.
    pub operator_type: OperatorType,
    /// Children in stored order. Order defines sequence execution and identifies
    /// the body (index zero) of a loop.
    pub children: Vec<Node>,
}

impl Operator {
    /// Creates an operator with no children for incremental construction.
    ///
    /// The empty operator is invalid until sufficient children are added.
    pub fn new(operator_type: OperatorType) -> Self {
        Self {
            operator_type,
            children: Vec::new(),
        }
    }
}

/// Errors encountered when modifying or validating a process tree.
#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
pub enum TreeError {
    /// Attempted to append a child to a terminal leaf.
    #[error("cannot add child to a leaf node")]
    LeafCannotHaveChildren,
    /// An operator has fewer children than required by [`Node::validate`].
    #[error("{operator:?} requires at least {minimum} children, got {actual}")]
    InvalidArity {
        /// The operator whose arity check failed.
        operator: OperatorType,
        /// Minimum number of children required for this operator.
        minimum: usize,
        /// Number of children present.
        actual: usize,
    },
}

/// An activity/silent leaf or an operator containing further nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    /// An internal node combining child behaviours.
    Operator(Operator),
    /// A terminal activity or silent step.
    Leaf(Leaf),
}

impl Node {
    /// Creates an empty operator node for incremental construction.
    ///
    /// Add children with [`Self::add_child`] before validation.
    pub fn new_operator(op_type: OperatorType) -> Self {
        Self::Operator(Operator::new(op_type))
    }

    /// Creates an operator node with children in the supplied order.
    ///
    /// Does not validate arity or descendants; use [`Self::validate`] or
    /// [`ProcessTree::new`] once the hierarchy is complete.
    pub fn new_operator_with_children(op: OperatorType, children: Vec<Node>) -> Self {
        Self::Operator(Operator {
            operator_type: op,
            children,
        })
    }

    /// Creates an activity leaf for `Some(activity)` or a silent leaf for `None`.
    pub fn new_leaf(leaf_label: Option<Activity>) -> Self {
        Self::Leaf(Leaf::new(leaf_label))
    }

    /// Appends a child to this operator, preserving existing child order.
    ///
    /// Does not validate the child or the resulting hierarchy.
    ///
    /// # Errors
    /// Returns [`TreeError::LeafCannotHaveChildren`] if this node is a leaf.
    pub fn add_child(&mut self, child: Node) -> Result<(), TreeError> {
        match self {
            Node::Operator(op) => {
                op.children.push(child);
                Ok(())
            }
            Node::Leaf(_) => Err(TreeError::LeafCannotHaveChildren),
        }
    }

    /// Checks this node and all descendants for valid operator arity.
    ///
    /// Leaves are always valid. Loops require at least two children (body and
    /// a redo branch); all other operators require at least one child.
    ///
    /// # Errors
    /// Returns [`TreeError::InvalidArity`] for the first invalid operator found,
    /// checking this node before its descendants in stored child order.
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

/// A process tree with a recursively validated root.
///
/// The root is accessible by shared reference, preserving the validation
/// performed during construction.
///
/// # Examples
/// ```
/// use inductive_miner::{Activity, Node, OperatorType, ProcessTree};
///
/// let root = Node::new_operator_with_children(
///     OperatorType::Sequence,
///     vec![
///         Node::new_leaf(Some(Activity::from("A"))),
///         Node::new_leaf(Some(Activity::from("B"))),
///     ],
/// );
/// let tree = ProcessTree::new(root)?;
/// tree.root().validate()?;
/// # Ok::<(), inductive_miner::TreeError>(())
/// ```
#[derive(Debug)]
pub struct ProcessTree {
    /// Root node, validated before the tree is constructed.
    root: Node,
}

impl ProcessTree {
    /// Validates `root` and takes ownership of it as a complete process tree.
    ///
    /// # Errors
    /// Returns the error from [`Node::validate`] if any operator has invalid arity.
    pub fn new(root: Node) -> Result<Self, TreeError> {
        root.validate()?;
        Ok(Self { root })
    }

    /// Borrows the validated root without permitting mutation.
    pub fn root(&self) -> &Node {
        &self.root
    }
}

/// An operator and an ordered partition of an event log's activity alphabet.
///
/// Mining splits the log into one sublog per partition and recursively mines
/// the corresponding child. Partition order defines sequence order; for loops,
/// the first partition is the body and remaining partitions are redo alternatives.
/// Order within each set is unspecified.
///
/// `A` defaults to named [`Activity`] values. Indexed mining uses
/// [`super::indexed_log::ActivityId`] instead. Constructing a cut does not
/// validate it; call [`Self::validate`] against the current log's alphabet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cut<A: Eq + std::hash::Hash = Activity> {
    /// Operator that combines the recursively mined children.
    pub operator: OperatorType,
    /// At least two nonempty, disjoint activity sets covering the log alphabet.
    pub partitions: Vec<HashSet<A>>,
}

/// Errors detected when checking a cut's activity partitions.
#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
pub enum CutError {
    /// The cut contains fewer than two partitions, or a partition is empty.
    #[error("a nontrivial cut requires at least two nonempty parts")]
    EmptyPartition,
    /// An activity occurs in more than one partition.
    #[error("cut parts overlap")]
    Overlap,
    /// The partition union omits log activities or includes activities outside it.
    #[error("cut parts must cover exactly the log activity alphabet")]
    AlphabetMismatch,
}

impl<A: Clone + Eq + std::hash::Hash> Cut<A> {
    /// Checks that partitions form a nontrivial partition of `alphabet`.
    ///
    /// Requires at least two nonempty parts, no overlap, and exact coverage.
    /// Does not check operator-specific behavioural requirements or whether a
    /// particular splitter supports the operator.
    ///
    /// # Errors
    /// Checks return [`CutError::EmptyPartition`], [`CutError::Overlap`], or
    /// [`CutError::AlphabetMismatch`], in that order of precedence.
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
