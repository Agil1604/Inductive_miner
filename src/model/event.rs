use super::Activity;

struct Event {
    activity: Activity
}

pub type Trace = Vec<Event>;

pub struct EventLog {
    events: Trace
}