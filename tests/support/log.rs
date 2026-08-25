use super::{Activity, Event, EventLog, Lifecycle, Trace};

///
/// Builds a log from activity-name sequences, preserving order and duplicates.
///
pub fn log(traces: &[&[&str]]) -> EventLog {
    EventLog {
        traces: traces
            .iter()
            .enumerate()
            .map(|(i, names)| Trace {
                case_id: i.to_string(),
                events: names
                    .iter()
                    .map(|name| Event {
                        activity: Activity::from(*name),
                        lifecycle: Lifecycle::Unknown,
                        timestamp: None,
                    })
                    .collect(),
            })
            .collect(),
    }
}
