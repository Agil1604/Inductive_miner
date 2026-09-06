use crate::framework::SplitLog;
use crate::{IndexedCut, IndexedEventLog, OperatorType, Trace};

///
/// XOR-split filtering strategy for splitting logs according to a cut.
///
#[derive(Debug, Clone, Copy, Default)]
pub struct LoopSplitFiltering;
impl SplitLog for LoopSplitFiltering {
    fn split_log(&self, log: &IndexedEventLog, cut: &IndexedCut) -> Vec<IndexedEventLog> {
        if cut.operator != OperatorType::Loop || cut.partitions.is_empty() {
            return Vec::new();
        }
        let mut traces = vec![Vec::new(); cut.partitions.len()];
        for trace in &log.traces {
            let mut part = 0;
            let mut events = Vec::new();
            for event in &trace.events {
                let Some(next) = cut
                    .partitions
                    .iter()
                    .position(|p| p.contains(&event.activity))
                else {
                    return Vec::new();
                };
                if next != part {
                    traces[part].push(Trace {
                        case_id: trace.case_id.clone(),
                        events: std::mem::take(&mut events),
                    });
                    part = next;
                }
                events.push(event.clone());
            }
            traces[part].push(Trace {
                case_id: trace.case_id.clone(),
                events,
            });
            if part != 0 {
                traces[0].push(Trace {
                    case_id: trace.case_id.clone(),
                    events: Vec::new(),
                });
            }
        }
        traces.into_iter().map(|t| log.with_traces(t)).collect()
    }
}
