pub mod directly_follows_graph;
pub mod event;
pub mod indexed_log;
pub mod process_tree;

/// A named activity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Activity(
    /// The observable activity name.
    pub String,
);

impl From<&str> for Activity {
    /// Copies a borrowed name into an owned activity label.
    fn from(name: &str) -> Self {
        Self(name.to_owned())
    }
}

impl From<String> for Activity {
    /// Takes ownership of a name without copying its string contents.
    fn from(name: String) -> Self {
        Self(name)
    }
}
