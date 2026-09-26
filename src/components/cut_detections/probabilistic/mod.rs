mod probabilities;
mod smt;
mod lifecycle;

pub use probabilities::ActivityRelations;
pub use lifecycle::ImclcCutFinder;

use super::{ConcurrentCut, CutChain, CutFinder, LoopCut, SequenceCut, XorCut};
use crate::framework::DetectCut;
use crate::{IndexedCut, IndexedEventLog, OperatorType};

/// A binary cut together with its accumulated probability.
#[derive(Debug, Clone)]
pub struct ScoredCut {
    /// Selected operator and ordered binary activity partition.
    pub cut: IndexedCut,
    /// Average crossing-pair score. This is the thesis's accumulated probability,
    /// not a calibrated probability that the whole discovered model is correct.
    pub probability: f64,
}

/// IMc cut finder. Empty traces and alphabets smaller than two bypass detection.
/// Tries standard IM cuts in XOR, sequence, concurrency, loop order first.
/// Only when no standard cut exists does it optimize a binary cut.
/// Scores within `1e-12` are treated as ties across optimization runs. Ties prefer
/// earlier alphabet activities on the left, then XOR, sequence, concurrency,
/// loop for the same partition.
#[derive(Debug, Default, Clone, Copy)]
pub struct ImcCutFinder;

impl ImcCutFinder {
    /// Optimizes binary partitions with Z3 using definitions 6.21 and 6.22.
    /// Each operator and left-partition size is solved separately.
    /// This is the probabilistic search in isolation, without the standard IM
    /// first pass. Use [`DetectCut::detect_cut`] for complete IMc cut detection.
    /// Concurrency requires start and end activities on both sides.
    /// Loop partitions place all observed starts and ends in the body.
    /// Returns `None` for empty-trace logs or alphabets with fewer than two
    /// activities. The returned cut uses the input log's interned activity IDs.
    ///
    /// # Panics
    /// Panics if Z3 reports an unknown result or cannot supply a model. Solver
    /// failures are not treated as absence of a cut.
    pub fn find_scored_cut(&self, log: &IndexedEventLog) -> Option<ScoredCut> {
        if log.traces.iter().any(|t| t.events.is_empty()) || log.alphabet().len() < 2 {
            return None;
        }
        let relations = ActivityRelations::from_log(log);
        smt::find_best(&relations)
    }
}

impl DetectCut for ImcCutFinder {
    /// Returns the first standard IM cut, or the best probabilistic cut if no
    /// standard cut exists. Standard cuts may have more than two partitions.
    ///
    /// # Panics
    /// Propagates solver failure panics from [`Self::find_scored_cut`].
    fn detect_cut(&self, log: &IndexedEventLog) -> Option<IndexedCut> {
        type StandardCuts =
            CutFinder<CutChain<XorCut, CutChain<SequenceCut, CutChain<ConcurrentCut, LoopCut>>>>;
        StandardCuts::default()
            .detect_cut(log)
            .or_else(|| self.find_scored_cut(log).map(|scored| scored.cut))
    }
}

/// Scores an ordered binary partition of local alphabet positions.
/// Returns `None` for unsupported operators, invalid partitions, or violated
/// operator boundary constraints. Loop redo boundary sets are optimized exactly.
///
/// `left` and `right` contain positions in [`ActivityRelations::alphabet`], not
/// raw activity IDs. XOR, sequence, and concurrency average the relevant pair
/// probabilities. Concurrency requires observed starts and ends on both sides.
/// For loops, `left` is the body and must contain all observed starts and ends;
/// a dynamic program optimizes nonempty redo start/end sets for this fixed cut.
/// This function evaluates a partition; it does not search for a partition.
pub fn score_partition(
    r: &ActivityRelations,
    left: &[usize],
    right: &[usize],
    op: OperatorType,
) -> Option<f64> {
    let mut covered = vec![false; r.alphabet.len()];
    if left.is_empty() || right.is_empty() {
        return None;
    }
    for &i in left.iter().chain(right) {
        if i >= covered.len() || covered[i] {
            return None;
        }
        covered[i] = true;
    }
    if covered.contains(&false) {
        return None;
    }
    let starts = |i: usize| r.graph.start_activities.contains_key(&r.alphabet[i]);
    let ends = |i: usize| r.graph.end_activities.contains_key(&r.alphabet[i]);
    if op == OperatorType::Concurrent
        && [left, right]
            .iter()
            .any(|part| !part.iter().any(|&i| starts(i)) || !part.iter().any(|&i| ends(i)))
    {
        return None;
    }
    let denominator = (left.len() * right.len()) as f64;
    if op != OperatorType::Loop {
        let index = match op {
            OperatorType::Xor => 0,
            OperatorType::Sequence => 1,
            OperatorType::Concurrent => 6,
            _ => return None,
        };
        return Some(
            left.iter()
                .flat_map(|&a| right.iter().map(move |&b| r.pair(a, b).0[index]))
                .sum::<f64>()
                / denominator,
        );
    }
    if right.iter().any(|&i| starts(i) || ends(i)) {
        return None;
    }

    let mut dp = [f64::NEG_INFINITY; 4];
    dp[0] = 0.0;
    for &b in right {
        let mut next = [f64::NEG_INFINITY; 4];
        for membership in 0..4 {
            let contribution = left
                .iter()
                .map(|&a| {
                    let p = r.pair(a, b).0;
                    let forward = membership & 1 != 0 && ends(a);
                    let reverse = membership & 2 != 0 && starts(a);
                    match (forward, reverse) {
                        (false, false) => p[3],
                        (true, false) => p[4],
                        (false, true) => p[5],
                        (true, true) => p[4] + p[5],
                    }
                })
                .sum::<f64>();
            for state in 0..4 {
                next[state | membership] = next[state | membership].max(dp[state] + contribution);
            }
        }
        dp = next;
    }
    Some(dp[3] / denominator)
}

#[cfg(test)]
mod tests;
