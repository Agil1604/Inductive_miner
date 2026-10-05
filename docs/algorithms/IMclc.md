# Inductive Miner – incompleteness, life cycle (IMclc)

IMclc combines [IMlc](IMlc.md)'s lifecycle-aware discovery with [IMc](IMc.md)'s probabilistic handling of incomplete observations. It first attempts IMlc cut detection. If no cut is found, it estimates activity relations using lifecycle executions and overlap evidence, then selects a binary cut with SMT optimization.

This document describes IMclc from section 6.5.3 of Sander Leemans' dissertation [*Robust Process Mining with Guarantees*](https://leemans.ch/publications/theses/phd.pdf). The relation probabilities and cut-scoring rules are inherited from section 6.3.

## Input and output

The input is an `EventLog` containing lifecycle events. IMclc uses event order, execution frequencies, completion-to-start relations, and observed execution overlap. Duplicate traces contribute separately to occurrence counts.

The output is a `ProcessTree`. The initial IMlc pass can discover XOR, sequence, concurrency, interleaving, and loop cuts. The probabilistic fallback considers binary XOR, sequence, concurrency, and loop cuts; it does not optimize interleaved or inclusive-choice candidates.

IMclc has no frequency-filtering threshold. Filtering splitters reconcile the original traces with inferred cuts, rather than removing graph edges according to IMf's frequency rule.

### Lifecycle assumptions

The same input conventions apply as in [IMlc](IMlc.md#lifecycle-assumptions): start/completion pairs describe executions, unmatched completions are accepted as atomic events, and unknown events are treated as atomic by library convention. Graph construction ignores enqueue and other transitions.

Prepare consistent input and remove ignored transitions before mining. The current pipeline does not automatically normalize the original input. Events are processed in their stored order, without timestamp sorting.

## Base cases

IMclc reuses `ImlcBaseCase`:

```text
WithoutEmptyTraces<BaseCaseChain<EmptyLog, SingleNonAtomicActivity>>
```

`EmptyLog` returns `τ` for a log without traces. `SingleNonAtomicActivity` returns an activity leaf when every trace contains one execution of the same activity. It accepts a start/completion pair or a lone atomic completion/unknown event.

IMclc does not use the approximate single-activity filtering base case from IMflc. Repeated executions are handled by subsequent stages rather than simplified according to a deviation threshold.

## Cut detection

`ImclcCutFinder` attempts the complete IMlc chain first:

```text
NonAtomicDfgCut<XorCut>
→ NonAtomicDfgCut<SequenceCut>
→ NonAtomicConcurrentCut
→ NonAtomicInterleavedCut
→ NonAtomicLoopCut
```

An applicable lifecycle cut is returned immediately and may have more than two partitions. Probabilistic optimization is attempted only when this entire chain fails.

The prose of section 6.5.3 explicitly specifies the IMlc first pass. The summary pseudocode shows only the highest-probability cut selection; this implementation follows the prose and retains the first pass.

Effective empty traces and alphabets smaller than two activities bypass probabilistic detection. See [IMlc.md](IMlc.md#cut-detection) for the first-pass rules and [architecture.md](../architecture.md) for framework contracts.

### Lifecycle relation evidence

`ActivityRelations::from_non_atomic_log` constructs the probability input from:

- the non-atomic DFG, including execution counts and trace boundaries;
- eventual completion-to-start ordering within individual traces;
- the concurrency graph of observed overlaps.

A start of `b` receives eventual evidence from activities already completed earlier in the trace. Unknown events and unmatched completions contribute as instantaneous atomic executions. An activity's start and matching completion are counted as one execution rather than two independent events.

Eventually-follows evidence is derived from actual traces, not graph reachability. Other than the overlap override, estimation uses IMc's seven relation classes and occurrence-based confidence rule.

### Overlap overrides other relations

If activities `a` and `b` overlap in the concurrency graph, the estimator classifies the pair as concurrent:

```text
p_concurrent(a, b) = 1
all other pair-relation probabilities = 0
```

This follows section 6.5.3's adaptation of activity relations. A direct observation of overlapping executions is stronger evidence for concurrency than interpreting lifecycle events as an ordinary atomic sequence.

```text
Trace: ⟨a_s, b_s, b_c, a_c⟩
Observed relation: a ∥ b
```

For pairs without observed overlap, probabilities follow Table 6.2 using lifecycle-aware direct and eventual evidence. The frequency parameter is based on execution counts:

```text
z = (execution_count(a) + execution_count(b)) / 2
```

See [IMc.md](IMc.md#probability-estimation) for residual probability allocation.

### Shared SMT optimization

IMclc reuses IMc's Z3 optimizer and cut-scoring rules. Boolean variables select a binary partition and loop redo boundaries. Concurrency requires start/end evidence on both sides; loop candidates place all observed starts and ends in the body.

Optimization runs separately for each operator and left-partition size. The implementation uses the same deterministic tie-breaking and `1e-12` score tolerance as IMc. See [IMc.md](IMc.md#smt-optimization) for details.

`ImclcCutFinder::find_scored_cut` exposes this probabilistic search alone. `DetectCut::detect_cut` includes the IMlc first pass.

## Log splitting

IMclc reuses `ImflcLogSplitter`: filtering splitting with consistency repair for sequence, interleaving, and loop outputs.

| Operator | Splitter | Consistency repair |
|---|---|---|
| XOR | `XorSplitFiltering` | No |
| Sequence | `SequenceSplitFiltering` | Yes |
| Concurrent | `ConcurrentSplit` | No |
| Interleaved | `InterleavedSplitFiltering` | Yes |
| Loop | `LoopSplitFiltering` | Yes |

Filtering may remove a completion while retaining its start or separate an execution across segments. `ConsistentSplit` repairs each resulting sublog by inserting a completion immediately after every unmatched start, following the preprocessing approach in section 2.3.2.

```text
Before repair: ⟨a_s, b_s, b_c⟩
After repair:  ⟨a_s, a_c, b_s, b_c⟩
```

Synthetic completions copy their start event's activity and timestamp. Existing events preserve order and metadata, and repaired sublogs share the original interner. Unmatched completions remain accepted as atomic events.

XOR projection removes all events of excluded activities together; concurrency projection does not discard events within its branch. The thesis therefore requires repair for the other three operators.

Repair restores the adopted consistency convention, but does not recover the missing event's actual completion time. Filtering and inserted completions can change the observed behaviour.

## Fall-throughs

IMclc uses the composition given in the thesis summary:

```text
EmptyTraces → ConcurrentFlowerModel
```

`EmptyTraces` retains optionality as `×(τ, IMclc(nonempty traces))`. It does not filter empty traces by frequency.

`ConcurrentFlowerModel` creates parallel optional repeatable branches according to maximum simultaneous execution counts. This covers self-overlap that an ordinary atomic flower model cannot express. See [IMlc.md](IMlc.md#concurrent-flower-model) for an example.

The activity-extraction and tau-loop strategies from IMlc are not included in IMclc's fall-through composition.

## Worked example

Consider:

```text
L = [⟨a_s, b_s, b_c, a_c⟩]
```

1. No base case applies because two activities are present.
2. IMlc detection observes overlap and finds a concurrent cut.
3. IMclc returns that first-pass cut without invoking SMT.
4. Projection gives `[⟨a_s, a_c⟩]` and `[⟨b_s, b_c⟩]`.
5. The lifecycle base case returns leaves `a` and `b`.

The result is:

```text
∧(a, b)
```

If the probabilistic search is invoked directly for this log, the overlap relation also gives the concurrent candidate a score of one. In normal mining, that search is unnecessary because the first pass succeeds.

For a log without an IMlc cut, the SMT stage compares possible binary explanations. As in IMc, boundary constraints can rule out an otherwise likely relation, and the selected cut may require filtering and repair before recursion.

## Using the library

```rust
use inductive_miner::{algorithms::IMclc, EventLog, Miner};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let log = EventLog::default(); // Replace with a normalized lifecycle log.
    let tree = IMclc::default().mine(&log)?;
    println!("{tree:#?}");
    Ok(())
}
```

For XES reading, see the [Quick start](../../README.md#quick-start). The default classifier reads `lifecycle:transition`. Keep the intended execution order and prepare consistent input before mining.

## Guarantees and limitations

- **Soundness:** the result has sound process tree semantics under the appropriate lifecycle interpretation.
- **Fitness:** IMclc does not guarantee that every observed lifecycle trace fits the result; inferred cuts, filtering, and consistency repair may change behaviour.
- **Rediscoverability:** the reference guarantee depends on the thesis's model-class and log assumptions. It does not ensure correct inference from arbitrary incomplete observations.

Observed overlap does not reveal every concurrent relation, and a high cut score is not calibrated confidence in the complete model. IMclc does not guarantee maximum precision.

The output uses ordinary named activity leaves. Lifecycle-aware interpretation is needed to evaluate starts, completions, and self-overlap. Atomic language checking alone cannot establish lifecycle fitness.

The probability matrix uses quadratic storage in the number of activities, and SMT optimization can be expensive. Solver unknown results or missing models currently cause a panic, as in IMc.

## Implementation locations

| Part | Source code |
|---|---|
| IMclc composition | [`src/algorithms/imclc/mod.rs`](../../src/algorithms/imclc/mod.rs) |
| Lifecycle probabilistic finder | [`src/components/cut_detections/probabilistic/lifecycle.rs`](../../src/components/cut_detections/probabilistic/lifecycle.rs) |
| Lifecycle relation construction | [`src/components/cut_detections/probabilistic/probabilities.rs`](../../src/components/cut_detections/probabilistic/probabilities.rs) |
| Shared SMT optimizer | [`src/components/cut_detections/probabilistic/smt.rs`](../../src/components/cut_detections/probabilistic/smt.rs) |
| Splitter composition | [`src/algorithms/imflc/mod.rs`](../../src/algorithms/imflc/mod.rs) |
| Consistency repair | [`src/components/log_splitting/consistency.rs`](../../src/components/log_splitting/consistency.rs) |
| Concurrent flower fallback | [`src/components/fall_throughs/non_atomic/concurrent_flower_model.rs`](../../src/components/fall_throughs/non_atomic/concurrent_flower_model.rs) |
| IMclc tests | [`src/algorithms/imclc/tests.rs`](../../src/algorithms/imclc/tests.rs) |
