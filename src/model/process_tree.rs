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
    pub fn new(label: Option<Activity>) -> Leaf {
        if let Some(activity) = label {
            Leaf { activity_label: LeafType::Activity(activity) }
        } else {
            Leaf { activity_label: LeafType::Tau }
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
    /// Exclusive choise
    Xor,
    /// Concurrency
    Concurrent,
    /// Loop
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
    pub fn new(operator_type: OperatorType) -> Operator {
        Operator { operator_type, children: Vec::new() }
    }
}

#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
pub enum TreeError {
    #[error("cannot add child to a leaf node")]
    LeafCannotHaveChildren,
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
    pub fn new_operator(op_type: OperatorType) -> Node {
        Node::Operator(Operator::new(op_type))
    }

    ///
    /// Create an operator node with children vector
    /// 
    pub fn new_operator_with_children(op: OperatorType, children: Vec<Node>) -> Self {
        Node::Operator(Operator { operator_type: op, children })
    }

    ///
    /// Create a leaf node
    /// 
    pub fn new_leaf(leaf_label: Option<Activity>) -> Node {
        Node::Leaf(Leaf::new(leaf_label))
    }

    ///
    /// Add one child node to an existing operator node
    /// 
    pub fn add_child(&mut self, child: Node)  -> Result<(), TreeError> {
        match self {
            Node::Operator(op) => {
                op.children.push(child);
                Ok(())
            }
            Node::Leaf(_) => {
                Err(TreeError::LeafCannotHaveChildren)
            }
        }
    }
}

///
/// Process tree
/// 
#[derive(Debug)]
pub struct ProcessTree {
    root: Node
}

impl ProcessTree {
    ///
    /// Create an instance of [`ProcessTree`]
    /// 
    pub fn new(root: Node) -> ProcessTree {
        ProcessTree { root }
    }
    
    ///
    /// Getter for root node of the process tree
    /// 
    pub fn root(&self) -> &Node {
        &self.root
    }
}


pub struct Cut {
    pub operator: OperatorType,
    pub partitions: Vec<HashSet<Activity>>,
}


#[cfg(test)]
mod tests {

}