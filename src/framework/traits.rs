use crate::{IndexedCut, IndexedEventLog, MinerError, Node};

///
/// A trait for miners to implement the base case of the recursion.
///
pub trait BaseCase {
    fn base_case(&self, log: &IndexedEventLog) -> Option<Node>;
}

///
/// A trait for miners to implement the cut detection step of the recursion.
///
pub trait DetectCut {
    fn detect_cut(&self, log: &IndexedEventLog) -> Option<IndexedCut>;
}

///
/// A trait for miners to implement the log splitting step of the recursion.
///
pub trait SplitLog {
    fn split_log(&self, log: &IndexedEventLog, cut: &IndexedCut) -> Vec<IndexedEventLog>;
}

///
/// A trait for miners to implement the fall-through step of the recursion.
///
pub trait FallThrough {
    fn fall_through(&self, context: &FallThroughContext<'_>) -> Result<Node, MinerError>;
}

///
/// Callbacks into the current configured miner.
///
pub struct FallThroughContext<'a> {
    pub log: &'a IndexedEventLog,
    pub recurse: &'a dyn Fn(&IndexedEventLog) -> Result<Node, MinerError>,
    pub find_cut: &'a dyn Fn(&IndexedEventLog) -> Option<IndexedCut>,
}
