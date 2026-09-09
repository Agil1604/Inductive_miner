use super::{ActivityRelations, ScoredCut, score_partition};
use crate::{IndexedCut, OperatorType};
use z3::{
    Optimize, SatResult,
    ast::{Bool, Int, Real},
};

/// Converts a finite probability to a decimal rational with 17 fractional digits.
/// Z3 uses exact real arithmetic on this representation; the input probability
/// was originally computed as `f64`, so this does not restore exact fractions.
fn weight(value: f64) -> Real {
    let digits = format!("{value:.17}").replace('.', "");
    Real::from_rational_str(&digits, "100000000000000000").unwrap()
}

/// Selects the best result over four operators and all nontrivial left sizes.
/// Scores within `1e-12` tie; earlier activities on the left win, followed by
/// operator order XOR, sequence, concurrency, loop. Returns `None` if no
/// operator/size combination admits a valid partition.
///
/// # Panics
/// Propagates solver failures from [`solve`].
pub(super) fn find_best(r: &ActivityRelations) -> Option<ScoredCut> {
    let n = r.alphabet.len();
    let operators = [
        OperatorType::Xor,
        OperatorType::Sequence,
        OperatorType::Concurrent,
        OperatorType::Loop,
    ];
    let mut best: Option<(ScoredCut, Vec<bool>, usize)> = None;
    for (rank, op) in operators.into_iter().enumerate() {
        for size in 1..n {
            let Some((candidate, sides)) = solve(r, op, size) else {
                continue;
            };
            let replace = best
                .as_ref()
                .is_none_or(|(previous, previous_sides, previous_rank)| {
                    if (candidate.probability - previous.probability).abs() <= 1e-12 {
                        sides > *previous_sides
                            || (sides == *previous_sides && rank < *previous_rank)
                    } else {
                        candidate.probability > previous.probability
                    }
                });
            if replace {
                best = Some((candidate, sides, rank));
            }
        }
    }
    best.map(|(cut, _, _)| cut)
}

/// Optimizes one operator with exactly `size` activities in the left partition.
///
/// Boolean variables select partition membership and loop redo boundaries.
/// Symmetric operators put the first activity on the left to avoid mirror cuts.
/// Boundary constraints match [`score_partition`]. Lexicographic secondary
/// objectives prefer earlier activities on the left after maximizing the score.
/// The result includes membership bits for deterministic comparison across runs.
/// Returns `None` when constraints are unsatisfiable. Callers supply a supported
/// operator and a size strictly between zero and the alphabet length.
///
/// # Panics
/// Panics if Z3 returns `Unknown`, fails to supply a usable model, or produces a
/// partition rejected by [`score_partition`].
fn solve(r: &ActivityRelations, op: OperatorType, size: usize) -> Option<(ScoredCut, Vec<bool>)> {
    let n = r.alphabet.len();
    let optimizer = Optimize::new();
    let sides: Vec<_> = (0..n)
        .map(|i| Bool::new_const(format!("left_{i}")))
        .collect();
    let cardinality: Vec<_> = sides
        .iter()
        .map(|b| b.ite(&Int::from_i64(1), &Int::from_i64(0)))
        .collect();
    optimizer.assert(Int::add(&cardinality).eq(Int::from_u64(size as u64)));
    if matches!(op, OperatorType::Xor | OperatorType::Concurrent) {
        optimizer.assert(&sides[0]);
    }
    let starts = |i: usize| r.graph.start_activities.contains_key(&r.alphabet[i]);
    let ends = |i: usize| r.graph.end_activities.contains_key(&r.alphabet[i]);
    if op == OperatorType::Concurrent {
        for boundary in [true, false] {
            let entries: Vec<_> = (0..n)
                .filter(|&i| if boundary { starts(i) } else { ends(i) })
                .collect();
            let left: Vec<_> = entries.iter().map(|&i| sides[i].clone()).collect();
            let right: Vec<_> = entries.iter().map(|&i| sides[i].not()).collect();
            optimizer.assert(Bool::or(&left));
            optimizer.assert(Bool::or(&right));
        }
    }
    let redo_start: Vec<_> = (0..n)
        .map(|i| Bool::new_const(format!("redo_start_{i}")))
        .collect();
    let redo_end: Vec<_> = (0..n)
        .map(|i| Bool::new_const(format!("redo_end_{i}")))
        .collect();
    if op == OperatorType::Loop {
        for i in 0..n {
            if starts(i) || ends(i) {
                optimizer.assert(&sides[i]);
            }
            optimizer.assert(redo_start[i].implies(sides[i].not()));
            optimizer.assert(redo_end[i].implies(sides[i].not()));
        }
        optimizer.assert(Bool::or(&redo_start));
        optimizer.assert(Bool::or(&redo_end));
    }
    let zero = weight(0.0);
    let mut terms = Vec::new();
    for a in 0..n {
        for b in 0..n {
            if a == b {
                continue;
            }
            let crosses = Bool::and(&[sides[a].clone(), sides[b].not()]);
            let p = r.pair(a, b).0;
            let contribution = if op == OperatorType::Loop {
                let forward = Bool::and(&[Bool::from_bool(ends(a)), redo_start[b].clone()]);
                let reverse = Bool::and(&[Bool::from_bool(starts(a)), redo_end[b].clone()]);
                let indirect = Bool::or(&[forward.clone(), reverse.clone()]).not();
                Real::add(&[
                    forward.ite(&weight(p[4]), &zero),
                    reverse.ite(&weight(p[5]), &zero),
                    indirect.ite(&weight(p[3]), &zero),
                ])
            } else {
                weight(
                    p[match op {
                        OperatorType::Xor => 0,
                        OperatorType::Sequence => 1,
                        OperatorType::Concurrent => 6,
                        _ => unreachable!(),
                    }],
                )
            };
            terms.push(crosses.ite(&contribution, &zero));
        }
    }

    optimizer.maximize(&Real::add(&terms));
    for side in &sides {
        optimizer.maximize(&side.ite(&Int::from_i64(1), &Int::from_i64(0)));
    }
    match optimizer.check(&[]) {
        SatResult::Unsat => return None,
        SatResult::Unknown => panic!(
            "Z3 could not optimize the IMc cut: {:?}",
            optimizer.get_reason_unknown()
        ),
        SatResult::Sat => {}
    }
    let model = optimizer
        .get_model()
        .expect("Z3 returned no model for a satisfiable cut");
    let chosen: Vec<_> = sides
        .iter()
        .map(|b| model.eval(b, true).unwrap().as_bool().unwrap())
        .collect();
    let left: Vec<_> = (0..n).filter(|&i| chosen[i]).collect();
    let right: Vec<_> = (0..n).filter(|&i| !chosen[i]).collect();
    let probability =
        score_partition(r, &left, &right, op).expect("SMT model violates cut constraints");
    Some((
        ScoredCut {
            probability,
            cut: IndexedCut {
                operator: op,
                partitions: vec![
                    left.into_iter().map(|i| r.alphabet[i]).collect(),
                    right.into_iter().map(|i| r.alphabet[i]).collect(),
                ],
            },
        },
        chosen,
    ))
}
