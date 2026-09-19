pub mod algorithms;
pub mod components;
pub mod framework;
pub mod model;
pub mod io;

pub use framework::miner::{InductiveMiner, Miner, MinerError};

pub use model::Activity;
pub use model::concurrency_graph::ConcurrencyGraph;
pub use model::directly_follows_graph::DirectlyFollowsGraph;
pub use model::event::{Event, EventClassifier, EventLog, Lifecycle, Trace};
pub use model::process_tree::{
    Cut, CutError, Leaf, LeafType, Node, Operator, OperatorType, ProcessTree, TreeError,
};

pub use components::filtering::{FilteringConfig, InvalidDeviationThreshold};

// Shared helpers compiled only for this crate's unit tests.
#[cfg(test)]
#[path = "../tests/support/log.rs"]
pub(crate) mod test_support;
#[cfg(test)]
use crate as library;
#[cfg(test)]
#[path = "../tests/support/im_fitness.rs"]
pub(crate) mod test_fitness;
pub use model::indexed_log::{ActivityId, ActivityInterner, IndexedEventLog};
pub type IndexedCut = Cut<ActivityId>;
pub type IndexedDfg = DirectlyFollowsGraph<ActivityId>;
pub type IndexedConcurrencyGraph = ConcurrencyGraph<ActivityId>;
