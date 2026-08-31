use crate::{ActivityId, IndexedCut, IndexedDfg, IndexedEventLog, OperatorType};
use std::collections::{HashMap, HashSet};

///
/// Returns the alphabet and DFG of an indexed log, or `None` if the log is empty or has fewer than two activities.
///
pub(super) fn context(log: &IndexedEventLog) -> Option<(Vec<ActivityId>, IndexedDfg)> {
    if log.traces.iter().any(|t| t.events.is_empty()) {
        return None;
    }
    let a = log.alphabet();
    if a.len() < 2 {
        return None;
    }
    Some((a, IndexedDfg::from_log(log)))
}

///
/// Returns the alphabet, DFG, and witnesses of an indexed log, or `None` if the log is empty or has fewer than two activities.
///
pub(super) fn components(
    a: &[ActivityId],
    connected: impl Fn(usize, usize) -> bool,
) -> Vec<HashSet<ActivityId>> {
    let mut seen = vec![false; a.len()];
    let mut parts = Vec::new();
    for i in 0..a.len() {
        if seen[i] {
            continue;
        }
        seen[i] = true;
        let mut stack = vec![i];
        let mut part = HashSet::new();
        while let Some(x) = stack.pop() {
            part.insert(a[x]);
            for (y, visited) in seen.iter_mut().enumerate() {
                if !*visited && connected(x, y) {
                    *visited = true;
                    stack.push(y);
                }
            }
        }
        parts.push(part);
    }
    parts
}

///
/// Returns an `IndexedCut` if the number of parts is at least two, otherwise returns `None`.
///
pub(super) fn cut(op: OperatorType, parts: Vec<HashSet<ActivityId>>) -> Option<IndexedCut> {
    (parts.len() >= 2).then_some(IndexedCut {
        operator: op,
        partitions: parts,
    })
}
///
/// Witnesses between closest repeated occurrences.
///
pub(super) fn witnesses(log: &IndexedEventLog) -> HashSet<(ActivityId, ActivityId)> {
    let mut best: HashMap<ActivityId, (usize, HashSet<ActivityId>)> = HashMap::new();
    for t in &log.traces {
        let mut previous = HashMap::new();
        for (i, e) in t.events.iter().enumerate() {
            if let Some(j) = previous.insert(e.activity, i) {
                let distance = i - j - 1;
                let entry = best
                    .entry(e.activity)
                    .or_insert((usize::MAX, HashSet::new()));
                if distance < entry.0 {
                    *entry = (distance, HashSet::new());
                }
                if distance == entry.0 {
                    entry
                        .1
                        .extend(t.events[j + 1..i].iter().map(|e| e.activity));
                }
            }
        }
    }
    best.into_iter()
        .flat_map(|(a, (_, bs))| bs.into_iter().map(move |b| (a, b)))
        .collect()
}
