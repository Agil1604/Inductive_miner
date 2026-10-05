# Inductive Miner – incompleteness (IMc)

IMc extends [IM](IM.md) to handle incomplete observations. When ordinary IM cannot find a cut, IMc estimates activity-relation probabilities and selects the highest-scoring binary cut using SMT optimization.

This document describes IMc from section 6.3 of Sander Leemans' dissertation [*Robust Process Mining with Guarantees*](https://leemans.ch/publications/theses/phd.pdf). The probability rules appear in Table 6.2, cut scoring in Definitions 6.21–6.22, and SMT-based detection in section 6.3.5.

## Input and output

The input is an `EventLog`. IMc uses activity labels, event order, occurrence counts, directly-follows relations, and eventually-follows relations. Duplicate traces contribute separately to occurrence counts.

The output is a `ProcessTree`. IMc can infer behaviour that was not observed, but this inference does not recover missing information with certainty.

IMc has no frequency-filtering threshold. Its filtering splitters handle events that conflict with an inferred cut; it does not apply IMf's DFG frequency filter.

## Base cases

IMc reuses `BaseCaseChain<EmptyLog, SingleActivity>` from IM:

- `EmptyLog` returns `τ` when the log contains no traces.
- `SingleActivity` returns an activity leaf when every trace contains exactly one event of the same activity.

Repeated executions do not trigger a filtering base case. Empty traces are handled by the fall-through. See [IM.md](IM.md#base-cases) for examples.

## Cut detection

`ImcCutFinder` first attempts the ordinary IM chain:

```text
XorCut → SequenceCut → ConcurrentCut → LoopCut
```

If any detector succeeds, its cut is returned immediately. These cuts can have more than two partitions. Only if the entire chain fails does probabilistic optimization begin. This first pass preserves IM's cut precedence and is explicitly discussed in the thesis's rediscoverability argument (section 6.3.4).

Logs containing empty traces or fewer than two activities bypass probabilistic detection. See [architecture.md](../architecture.md) for the framework contracts.

### Observed activity relations

For each ordered pair of distinct activities `(a, b)`, the estimator considers:

- `a → b`: `b` immediately follows `a` in some trace;
- `b → a`: the reverse directly-follows relation;
- `a ≺ b`: `a` eventually precedes `b` somewhere some trace;
- `b ≺ a`: the reverse eventually-follows relation.

Eventually-follows evidence comes from actual traces, not DFG reachability.
A path `a → c → b` assembled from different traces does not establish `a ≺ b`.

The evidence distinguishes seven relations:

| Relation            | Interpretation                                                         |
| ------------------- | ---------------------------------------------------------------------- |
| XOR                 | The activities have no observed eventual ordering in either direction. |
| Forward sequence    | Evidence supports `a` before `b`.                                      |
| Reverse sequence    | Evidence supports `b` before `a`.                                      |
| Indirect loop       | Both eventual directions occur without direct connections.             |
| Forward direct loop | `a → b` occurs and `b ≺ a` occurs.                                     |
| Reverse direct loop | `b → a` occurs and `a ≺ b` occurs.                                     |
| Concurrency         | Both directly-follows directions occur.                                |

These are activity-pair relations used for scoring; a direct-loop relation is not itself a complete process-tree loop.

### Probability estimation

Let `count(a)` and `count(b)` be event occurrence counts across the current log:

```text
z = (count(a) + count(b)) / 2
```

For an observed relation with possible stronger alternatives, the observed relation receives `1 − 1/(z + 1)`. The residual `1/(z + 1)` is divided equally among the compatible stronger relations specified by Table 6.2. Contradicted relations receive zero. Fully observed concurrency receives probability one.

For example, if only `a → b` is observed and no reverse eventual evidence exists, forward sequence receives the main weight. Forward direct loop and concurrency share the residual. More observations increase confidence in the observed relation according to this estimator.

Pair probabilities sum to one. They express the thesis's relation model, not calibrated confidence that the discovered process tree is correct.

### Scoring binary cuts

The optimizer partitions the alphabet into nonempty sets `A` and `B` and considers XOR, forward sequence, concurrency, and loop candidates.

For XOR, sequence, and concurrency, the score averages the corresponding relation probabilities over all crossing pairs:

```text
score(operator, A, B) = sum(p_operator(a, b)) / (|A| × |B|)
                       for a in A and b in B
```

Concurrent candidates require an observed start and end activity on each side. For a loop, `A` is the body and must contain all observed start and end activities.

Loop scoring also chooses nonempty redo-start and redo-end sets within `B`. For each crossing pair, it uses direct-loop evidence at the required boundaries and indirect-loop evidence elsewhere. When both boundary connections apply, both direct-loop contributions are included. 

### SMT optimization

The implementation uses Z3 `Optimize` rather than enumerating all activity partitions in Rust. Boolean variables select partition membership and loop redo boundaries. Constraints enforce nonempty partitions and operator boundaries.

Each operator is optimized separately for each possible left-partition size. This makes the averaging denominator constant within one optimization run. The best result is then selected across runs.

For reproducibility, scores within `1e-12` are treated as ties. Ties prefer earlier alphabet activities on the left, then XOR, sequence, concurrency, and loop for the same partition. Symmetric XOR and concurrent candidates fix the first activity on the left to avoid mirror duplicates.

SMT optimization avoids explicit exhaustive partition enumeration, but difficult instances can still be expensive. If Z3 returns unknown or cannot provide a model, the current implementation panics rather than interpreting the failure as an absent cut.

## Log splitting

IMc uses the same four splitters as IMf, even when the ordinary IM first pass finds the cut.

These splitters reconcile the original log with the inferred cut. They can remove observed behaviour, which is why IMc does not guarantee perfect fitness. See [IMf.md](IMf.md#log-splitting) for the individual splitting rules.

## Fall-throughs

IMc uses a smaller fall-through composition than IM:

```text
EmptyTraces → FlowerModel
```

`EmptyTraces` retains optionality as `×(τ, IMc(nonempty traces))`. It does not filter empty traces by frequency. Recursive calls use the current IMc miner.

`FlowerModel` supplies the final fallback, allowing arbitrary nonempty sequences of the remaining activities. The activity-extraction and tau-loop fall-throughs from IM are not included in this composition.

## Worked example

Consider the log:

```text
L = [⟨a, a, b, a, b, b⟩]
```

1. The single-activity base case does not apply.
2. The DFG is connected and has edges in both directions, preventing XOR and sequence cuts.
3. The ordinary concurrent cut fails its trace-boundary requirements: `a` is the only start activity and `b` the only end activity.
4. The loop body must contain both activities, leaving no separate redo part.
5. IMc therefore estimates relations and runs binary SMT optimization.

Both directly-follows directions make the pair's estimated concurrency probability one. However, the concurrent candidate violates the boundary constraints, as does the loop candidate. The remaining XOR and sequence candidates have zero scores; the deterministic tie rule selects XOR with `{a}` on the left.

The XOR splitter assigns the trace to the first branch because both activities occur three times. It retains `⟨a, a, a⟩` and removes the `b` events; recursion models the retained repetition.

This deliberately small example exposes a limitation: strong pair evidence may have no compatible complete cut, leaving a low-scoring result. IMc does not promise that every inferred cut is a convincing explanation of the input.

## Using the library

```rust
use inductive_miner::{algorithms::IMc, EventLog, Miner};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let log = EventLog::default(); // Replace with your event log.
    let tree = IMc::default().mine(&log)?;
    println!("{tree:#?}");
    Ok(())
}
```

For reading XES input, see the [Quick start](../../README.md#quick-start). IMc interprets atomic event order. Use IMclc for lifecycle-aware probabilistic detection with observed execution overlap.

`ImcCutFinder::find_scored_cut` exposes the probabilistic search in isolation;
`DetectCut::detect_cut` includes the ordinary IM first pass.

## Guarantees and limitations

- **Soundness:** the result is a process tree with sound standard semantics.
- **Fitness:** IMc does not guarantee that every observed trace fits the result,
  because inferred cuts and filtering splitters can remove behaviour.
- **Rediscoverability:** the thesis establishes guarantees under its model-class
  and log assumptions. The ordinary IM first pass contributes to this property.

IMc does not guarantee maximum precision or correct reconstruction from arbitrary incomplete logs. More observations change the estimates, but cannot establish which unobserved behaviour actually exists in the source process.

The probability matrix requires quadratic storage in the number of activities, and SMT optimization adds computational cost. IMc prioritizes handling incompleteness rather than the scalability of DFG-only variants.

## Implementation locations

| Part                          | Source code                                                                                                                          |
| ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| IMc composition               | [`src/algorithms/imc/mod.rs`](../../src/algorithms/imc/mod.rs)                                                                       |
| Cut finder and cut scoring    | [`src/components/cut_detections/probabilistic/mod.rs`](../../src/components/cut_detections/probabilistic/mod.rs)                     |
| Relation probabilities        | [`src/components/cut_detections/probabilistic/probabilities.rs`](../../src/components/cut_detections/probabilistic/probabilities.rs) |
| SMT optimization              | [`src/components/cut_detections/probabilistic/smt.rs`](../../src/components/cut_detections/probabilistic/smt.rs)                     |
| Probabilistic component tests | [`src/components/cut_detections/probabilistic/tests.rs`](../../src/components/cut_detections/probabilistic/tests.rs)                 |
| IMc tests                     | [`src/algorithms/imc/tests.rs`](../../src/algorithms/imc/tests.rs)                                                                   |
