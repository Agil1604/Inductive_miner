use crate::{ActivityId, IndexedEventLog, Trace};

///
/// Projects traces onto a subset of activities, retaining empty projections.
///
pub(crate) fn project(
    log: &IndexedEventLog,
    keep: impl Fn(&ActivityId) -> bool,
) -> IndexedEventLog {
    log.with_traces(
        log.traces
            .iter()
            .map(|t| Trace {
                case_id: t.case_id.clone(),
                events: t
                    .events
                    .iter()
                    .filter(|e| keep(&e.activity))
                    .cloned()
                    .collect(),
            })
            .collect(),
    )
}
