use crate::{ActivityId, IndexedDfg, IndexedEventLog};
use std::collections::{HashMap, HashSet};

/// Estimated relation probabilities for an ordered pair of distinct activities.
///
/// Entries sum to one for distinct activity pairs. Diagonal entries in the
/// stored matrix are zero because cuts never compare an activity with itself.
#[derive(Debug, Clone, Copy)]
pub struct PairProbabilities(
    /// Entries: XOR (0), forward sequence (1), reverse sequence (2), indirect
    /// loop (3), forward direct loop (4), reverse direct loop (5), concurrency (6).
    pub [f64; 7],
);

/// Pairwise evidence and probabilities for one recursive sublog.
///
/// The matrix uses local alphabet positions. Keep the public alphabet and graph
/// consistent with their original values: changing them does not recompute the
/// matrix and can invalidate scoring.
pub struct ActivityRelations {
    /// Present activity IDs in deterministic ascending order.
    pub alphabet: Vec<ActivityId>,
    /// Occurrence frequencies, direct succession, and trace boundaries.
    pub graph: IndexedDfg,
    /// Probability matrix indexed by positions in `alphabet`.
    probabilities: Vec<Vec<PairProbabilities>>,
}

impl ActivityRelations {
    /// Builds the DFG, eventually-follows evidence, and pair probability matrix.
    ///
    /// Eventually-follows means that one activity occurs earlier than another
    /// in the same trace; it is not inferred from graph reachability. Frequencies
    /// count all events and duplicate traces. Metadata is ignored and events are
    /// neither reordered nor filtered by lifecycle. Work and storage for the
    /// probability matrix are quadratic in the number of distinct activities.
    pub fn from_log(log: &IndexedEventLog) -> Self {
        let alphabet = log.alphabet();
        let graph = IndexedDfg::from_log(log);
        let positions: HashMap<_, _> = alphabet.iter().enumerate().map(|(i, &a)| (a, i)).collect();
        let mut eventually = HashSet::new();
        for trace in &log.traces {
            let mut seen = HashSet::new();
            for event in &trace.events {
                for &a in &seen {
                    eventually.insert((a, event.activity));
                }
                seen.insert(event.activity);
            }
        }
        let n = alphabet.len();
        let mut probabilities = vec![vec![PairProbabilities([0.0; 7]); n]; n];
        for &a in &alphabet {
            for &b in &alphabet {
                if a == b {
                    continue;
                }
                let z = (graph.activities[&a] as f64 + graph.activities[&b] as f64) / 2.0;
                probabilities[positions[&a]][positions[&b]] = estimate(
                    graph.edges.contains_key(&(a, b)),
                    graph.edges.contains_key(&(b, a)),
                    eventually.contains(&(a, b)),
                    eventually.contains(&(b, a)),
                    z,
                );
            }
        }
        Self {
            alphabet,
            graph,
            probabilities,
        }
    }

    /// Looks up probabilities using local alphabet positions, not interned IDs.
    /// The ordered pair is `(alphabet[a], alphabet[b])`.
    ///
    /// # Panics
    /// Panics if either position is outside the stored matrix.
    pub fn pair(&self, a: usize, b: usize) -> PairProbabilities {
        self.probabilities[a][b]
    }
}

/// Selects a row from direct (`ab`, `ba`) and eventual evidence.
///
/// `z` is the average occurrence count of the two activities. The observed
/// relation receives `1 - 1/(z + 1)` and possible stronger relations share the
/// residual; contradicted relations receive zero. Fully observed concurrency
/// receives probability one. Inputs are expected to describe distinct observed
/// activities, so `z >= 1` and direct evidence implies eventual evidence.
pub(super) fn estimate(
    ab: bool,
    ba: bool,
    eventual_ab: bool,
    eventual_ba: bool,
    z: f64,
) -> PairProbabilities {
    let (observed, stronger): (usize, &[usize]) = match (ab, ba, eventual_ab, eventual_ba) {
        (true, true, _, _) => (6, &[]),
        (true, false, _, true) => (4, &[6]),
        (false, true, true, _) => (5, &[6]),
        (true, false, _, false) => (1, &[4, 6]),
        (false, true, false, _) => (2, &[5, 6]),
        (false, false, true, true) => (3, &[4, 5, 6]),
        (false, false, true, false) => (1, &[3, 4, 5, 6]),
        (false, false, false, true) => (2, &[3, 4, 5, 6]),
        (false, false, false, false) => (0, &[1, 2, 3, 4, 5, 6]),
    };
    let mut p = [0.0; 7];
    if stronger.is_empty() {
        p[observed] = 1.0;
    } else {
        let residual = 1.0 / (z + 1.0);
        p[observed] = 1.0 - residual;
        for &i in stronger {
            p[i] = residual / stronger.len() as f64;
        }
    }
    PairProbabilities(p)
}
