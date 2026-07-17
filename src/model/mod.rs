pub mod event;
pub mod process_tree;

///
/// Represents an activity in a process model
/// 
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Activity(pub String);
