use crate::framework::SplitLog;
use crate::{ActivityId, Event, IndexedCut, IndexedEventLog, OperatorType, Trace};

/// Splits deviating interleaved traces using divide-and-conquer.
///
/// For each trace, selects a partition and interval maximizing the number of
/// events belonging to that partition minus the number of other events inside
/// the interval. Keeps matching events within the interval and discards that
/// partition's events outside it. Recursively processes the two outer sequences
/// after removing events of the selected partition. Equal scores retain the
/// first interval encountered during the left-to-right scan.
///
/// Returns one sublog per cut partition in partition order, with one trace per
/// input case, including empty projections. Retained events preserve their
/// order, lifecycle, and timestamps; case IDs, duplicate cases, and shared
/// activity interning are preserved. If a partition is selected in both outer
/// recursions, the later result replaces the earlier one, following the thesis's
/// `splitTrace` assignment.
///
/// Assumes a validated cut covering the input alphabet. Returns an empty vector
/// for an operator other than [`OperatorType::Interleaved`] or no partitions.
/// Used with [`crate::components::cut_detections::InterleavedCutFiltering`];
/// discarding events means this splitter does not guarantee fitness.
///
/// # Panics
/// Panics if an input event's activity belongs to no cut partition.
#[derive(Debug, Default, Clone, Copy)]
pub struct InterleavedSplitFiltering;
impl SplitLog for InterleavedSplitFiltering {
    fn split_log(&self, log: &IndexedEventLog, cut: &IndexedCut) -> Vec<IndexedEventLog> {
        if cut.operator != OperatorType::Interleaved || cut.partitions.is_empty() {
            return Vec::new();
        }
        let mut traces = vec![Vec::new(); cut.partitions.len()];
        for trace in &log.traces {
            let mut subtraces = vec![Vec::new(); cut.partitions.len()];
            split_trace(&trace.events, cut, &mut subtraces);
            for (i, events) in subtraces.into_iter().enumerate() {
                traces[i].push(Trace {
                    case_id: trace.case_id.clone(),
                    events,
                });
            }
        }
        traces
            .into_iter()
            .map(|traces| log.with_traces(traces))
            .collect()
    }
}

fn split_trace(
    events: &[Event<ActivityId>],
    cut: &IndexedCut,
    output: &mut [Vec<Event<ActivityId>>],
) {
    if events.is_empty() {
        return;
    }
    let n = cut.partitions.len();
    let mut values = vec![0isize; n];
    let mut starts = vec![0; n];
    let mut best = (0, 0, 0);
    let mut best_value = -1;
    for (position, event) in events.iter().enumerate() {
        let s = cut
            .partitions
            .iter()
            .position(|p| p.contains(&event.activity))
            .expect("event outside cut alphabet");
        if values[s] < 0 {
            values[s] = 1;
            starts[s] = position;
        } else {
            values[s] += 1;
        }
        for (i, value) in values.iter_mut().enumerate() {
            if i != s {
                *value -= 1;
            }
        }
        if values[s] > best_value {
            best_value = values[s];
            best = (s, starts[s], position + 1);
        }
    }
    let (part, start, end) = best;
    let keep = |e: &&Event<ActivityId>| !cut.partitions[part].contains(&e.activity);
    let left: Vec<_> = events[..start].iter().filter(keep).cloned().collect();
    let right: Vec<_> = events[end..].iter().filter(keep).cloned().collect();
    split_trace(&left, cut, output);
    output[part] = events[start..end]
        .iter()
        .filter(|e| cut.partitions[part].contains(&e.activity))
        .cloned()
        .collect();
    split_trace(&right, cut, output);
}
