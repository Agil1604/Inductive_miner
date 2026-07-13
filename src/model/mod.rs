pub mod event;
pub mod process_tree;

pub use event::EventLog;
pub use process_tree::{ProcessTree, Cut, Node};

pub type Activity = String;
