pub mod model;
pub mod utils;

pub use model::Activity;
pub use model::event::{Event, EventClassifier, Lifecycle, Trace, EventLog};
pub use model::process_tree::{CutError, Leaf, LeafType, Operator, OperatorType, TreeError, Cut, Node, ProcessTree};

pub use utils::read_xes::{read_xes, read_xes_from_reader};