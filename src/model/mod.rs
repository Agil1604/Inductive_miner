pub mod directly_follows_graph;
pub mod event;
pub mod process_tree;

///
/// Represents an activity in a process model
///
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Activity(pub String);

impl From<&str> for Activity {
    fn from(name: &str) -> Self {
        Self(name.to_owned())
    }
}

impl From<String> for Activity {
    fn from(name: String) -> Self {
        Self(name)
    }
}
