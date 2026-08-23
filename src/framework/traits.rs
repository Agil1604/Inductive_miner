use crate::{Cut, EventLog, Node};

///
/// A trait for miners to implement the base case of the recursion.
///
pub trait BaseCase {
    fn base_case(&self, log: &EventLog) -> Option<Node>;
}

///
/// A trait for miners to implement the cut detection step of the recursion.
///
pub trait DetectCut {
    fn detect_cut(&self, log: &EventLog) -> Option<Cut>;
}


///
/// A trait for miners to implement the log splitting step of the recursion.
///
pub trait SplitLog {
    fn split_log(&self, log: &EventLog, cut: &Cut) -> Vec<EventLog>;
}

///
/// A trait for miners to implement the fall-through step of the recursion.
///
pub trait FallThrough {
    fn fall_through(&self, log: &EventLog) -> Node;
}
