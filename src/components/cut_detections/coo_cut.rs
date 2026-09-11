use super::{ConcurrentCut, CutContext, CutStrategy};
use crate::{IndexedCut, OperatorType};
use std::collections::HashSet;

/// Distinguishes concurrency and inclusive choice using coo relations.
///
/// Starts with the partition returned by [`ConcurrentCut`]. Each input trace is
/// then represented by the set of parts whose activities occur in it. This
/// activity-set language ignores event order and duplicate presence patterns.
///
/// While more than two parts remain, the first pair related by concurrency,
/// concurrent optionality, or interchangeability is merged. The presence
/// language is rebuilt after each merge. Implication means that whenever one
/// part occurs, another occurs too; optionality requires a matching presence
/// pattern without the optional part. Interchangeability requires patterns with
/// either part alone and both together, keeping the other parts unchanged.
///
/// Once merging stops, any remaining interchangeable pair selects
/// [`OperatorType::InclusiveChoice`]; otherwise the result is
/// [`OperatorType::Concurrent`]. Returns `None` if no initial concurrency cut
/// exists. The corresponding splitter must be
/// [`crate::components::log_splitting::InclusiveChoiceSplit`] or
/// [`crate::components::log_splitting::ConcurrentSplit`], respectively.
#[derive(Debug, Default, Clone, Copy)]
pub struct CooCut;
impl CutStrategy for CooCut {
    fn detect(&self, context: &CutContext<'_>) -> Option<IndexedCut> {
        let mut parts = ConcurrentCut.detect(context)?.partitions;
        loop {
            let language: HashSet<Vec<bool>> = context
                .log()
                .traces
                .iter()
                .map(|t| {
                    parts
                        .iter()
                        .map(|p| t.events.iter().any(|e| p.contains(&e.activity)))
                        .collect()
                })
                .collect();
            let implies = |a: usize, b: usize| language.iter().all(|t| !t[a] || t[b]);
            let optional = |a: usize| {
                language.iter().all(|t| {
                    if !t[a] {
                        return true;
                    }
                    let mut without = t.clone();
                    without[a] = false;
                    language.contains(&without)
                })
            };
            let interchangeable = |a: usize, b: usize| {
                language.iter().all(|t| {
                    if !t[a] && !t[b] {
                        return true;
                    }
                    [(true, true), (true, false), (false, true)]
                        .into_iter()
                        .all(|(x, y)| {
                            let mut variant = t.clone();
                            variant[a] = x;
                            variant[b] = y;
                            language.contains(&variant)
                        })
                })
            };
            let optional_concurrent = |a: usize, b: usize| {
                optional(a)
                    && implies(a, b)
                    && !(0..parts.len()).any(|c| {
                        c != a
                            && c != b
                            && !implies(b, c)
                            && implies(c, b)
                            && parts[a].union(&parts[b]).all(|&x| {
                                parts[c].iter().all(|&y| {
                                    context.dfg().edges.contains_key(&(x, y))
                                        && context.dfg().edges.contains_key(&(y, x))
                                })
                            })
                    })
            };
            let mut pair = None;
            let mut inclusive = false;
            'pairs: for a in 0..parts.len() {
                for b in a + 1..parts.len() {
                    let or = interchangeable(a, b);
                    inclusive |= or;
                    if parts.len() > 2
                        && (or
                            || (implies(a, b) && implies(b, a))
                            || optional_concurrent(a, b)
                            || optional_concurrent(b, a))
                    {
                        pair = Some((a, b));
                        break 'pairs;
                    }
                }
            }
            if let Some((a, b)) = pair {
                let moved = parts.remove(b);
                parts[a].extend(moved);
            } else {
                return Some(IndexedCut {
                    operator: if inclusive {
                        OperatorType::InclusiveChoice
                    } else {
                        OperatorType::Concurrent
                    },
                    partitions: parts,
                });
            }
        }
    }
}
