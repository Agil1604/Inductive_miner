# Inductive Miner – infrequent, life cycle (IMflc)

IMflc combines [IMlc](IMlc.md)'s lifecycle-aware discovery with [IMf](IMf.md)'s frequency filtering. It first attempts lifecycle cut detection, then retries on a filtered non-atomic DFG. Filtering splitters handle deviations, and consistency repair restores unmatched starts in their output.

This document describes IMflc from section 6.5.3 of Sander Leemans' dissertation [*Robust Process Mining with Guarantees*](https://leemans.ch/publications/theses/phd.pdf).

## Input and output

The input is an `EventLog` containing lifecycle events. IMflc uses activity execution order, frequencies, and observed overlap. Duplicate traces contribute separately to frequency counts.

The output is a `ProcessTree` supporting XOR, sequence, concurrency, interleaving, and loop cuts. IMflc does not detect inclusive choice. Leaves contain ordinary activity names and require lifecycle-aware interpretation to represent starts, completions, and self-overlap.

`FilteringConfig` supplies a finite threshold `f` in `[0, 1]`, defaulting to `0.2`. The same value controls DFG filtering, the single-activity estimate, and empty-trace handling throughout recursion. It is not a global percentage of events to remove.

### Lifecycle assumptions

Input conventions follow [IMlc](IMlc.md#lifecycle-assumptions): start/completion pairs describe executions; unmatched completions and unknown events are treated as atomic; graph construction ignores enqueue and other transitions.

Prepare consistent input and remove unsupported transitions before mining. Repair after splitting does not replace input normalization. Events retain their stored order; timestamps are not used to sort them or infer overlap.

## Base cases

IMflc composes:

```text
WithoutEmptyTraces<BaseCaseChain<EmptyLog, SingleNonAtomicActivityFiltering>>
```

### Empty log

`EmptyLog` returns `τ` when no traces are recorded. Logs containing empty traces are deferred to filtering fall-through handling.

### Approximately one execution per trace

`SingleNonAtomicActivityFiltering` applies IMf's single-activity estimate to executions rather than counting starts and completions as separate events. The implementation repairs unmatched starts for this estimate, then projects to completion and unknown events.

For a projected log containing exactly one activity, with `t` traces and `e` executions, the base case applies when:

```text
|t / (e + t) − 0.5| ≤ f
```

Equality is accepted. A matching start/completion pair contributes one execution:

```text
L = [⟨a_s, a_c⟩]
p = 1 / (1 + 1) = 0.5
Result: a
```

With two executions of `a` in one trace, `p = 1/3`. The default threshold `0.2` can simplify these repetitions to one activity leaf. The estimate is aggregate; it does not independently require exactly one execution in every trace.

This component can discard observed repetition, so its result does not guarantee fitness for every original trace.

## Cut detection

`NonAtomicFilteringCutFinder` prepares the lifecycle abstractions and attempts two chains. The original concurrency graph is retained in both passes; the second pass filters only the non-atomic DFG.

### First pass: ordinary IMlc detection

```text
NonAtomicDfgCut<XorCut>
→ NonAtomicDfgCut<SequenceCut>
→ NonAtomicConcurrentCut
→ NonAtomicInterleavedCut
→ NonAtomicLoopCut
```

An applicable cut is returned immediately. The filtering pass runs only when the entire first chain fails. See [IMlc.md](IMlc.md#cut-detection) for lifecycle footprints and overlap protection.

### Second pass: filtered lifecycle detection

```text
NonAtomicDfgCut<XorCut>
→ NonAtomicDfgCut<SequenceCut>
→ NonAtomicConcurrentCut
→ NonAtomicInterleavedCutFiltering
→ NonAtomicLoopCut
```

Filtering follows [IMf's relative-frequency rule](IMf.md#relative-frequency-filtering): an edge is retained when its count is at least `f` times the maximum outgoing count of its source. End boundaries participate in source maxima; start boundaries share an artificial source. Equality retains the relation.

Activity vertices and frequencies remain present. The replacement context resets its cached DFG adapter, so the second pass uses filtered succession edges with the original overlap evidence. Observed overlaps continue to prevent XOR or sequence partitions from separating concurrent executions.

Unlike IMfa's filtered chain, this chain retains lifecycle concurrency detection. It uses ordinary `SequenceCut`, rather than `SequenceCutStrict`.

### Filtered interleaving

`NonAtomicInterleavedCutFiltering` applies the lifecycle interleaving footprint without the original-trace block-merging pass. Deviating returns to an earlier part can therefore survive detection and be repaired by the matching filtering splitter.

The ordinary lifecycle detector enforces complete blocks by merging parts. The filtered variant instead leaves event removal to `InterleavedSplitFiltering`.

## Log splitting

IMflc splits the original log, including when the unfiltered first pass finds the cut. It uses filtering splitters and repairs selected outputs:

| Operator | Splitter | Consistency repair |
|---|---|---|
| XOR | `XorSplitFiltering` | No |
| Sequence | `SequenceSplitFiltering` | Yes |
| Concurrent | `ConcurrentSplit` | No |
| Interleaved | `InterleavedSplitFiltering` | Yes |
| Loop | `LoopSplitFiltering` | Yes |

The basic filtering rules are described in [IMf.md](IMf.md#log-splitting). Interleaved splitting uses the divide-and-conquer interval selection described in [IMfa.md](IMfa.md#divide-and-conquer-interleaved-splitting).

Retained events preserve their order and metadata. Output logs share the parent's activity interner. The extended dispatcher also supports inclusive choice, but the IMflc finder does not produce that operator.

### Consistency repair

Filtering can retain a start while removing its completion, or separate them into different loop segments. `ConsistentSplit` wraps sequence, interleaved, and loop splitters and repairs every resulting sublog.

Following section 2.3.2, each unmatched start receives a synthetic completion immediately after it:

```text
Before repair: ⟨a_s, b_s, b_c⟩
After repair:  ⟨a_s, a_c, b_s, b_c⟩
```

Synthetic completions copy their start's activity and timestamp. Existing events remain in their original order, and unmatched completions remain accepted as atomic executions. The repair is idempotent for the adopted matching convention.

XOR removes events of excluded activities together, while concurrency projection retains events belonging to its branch. The thesis therefore requires repair for the other three operators.

Repair restores consistency under this convention, but does not reconstruct the missing completion's actual time or the original execution overlap.

## Fall-throughs

IMflc tries `EmptyTracesFiltering` before the complete IMlc fall-through composition:

```text
EmptyTracesFiltering
→ EmptyTraces
→ NonAtomicActivityOncePerTrace
→ ActivityConcurrent
→ StrictNonAtomicTauLoop
→ NonAtomicTauLoop
→ ConcurrentFlowerModel
```

`EmptyTracesFiltering` removes empty traces and recursively mines the remainder. It retains an XOR branch with `τ` only when the empty-trace fraction is at least `f`; equality retains optionality. A log consisting entirely of empty traces returns `τ`.

When empty traces occur, this component already returns a result. The nested ordinary `EmptyTraces` belongs to the reused IMlc composition and is not an additional filtering pass.

The remaining lifecycle components preserve execution consistency when extracting activities or splitting repetitions. The concurrent flower fallback supports simultaneous executions of the same activity. See [IMlc.md](IMlc.md#fall-throughs) for their rules.

## Worked example

Consider:

```text
L = [⟨a_s, a_c, a_s, a_c⟩]
f = 0.2
```

1. No trace is empty, so guarded base-case detection proceeds.
2. The execution projection contains `[⟨a, a⟩]`, with `t = 1` and `e = 2`.
3. The estimate is `p = 1/3`.
4. Since `|1/3 − 0.5| ≈ 0.167 ≤ 0.2`, the filtering base case applies.

The result is:

```text
a
```

The observed repetition is simplified. With `f = 0`, this base case does not apply, and lifecycle tau-loop handling can discover `loop(a, τ)` instead.

Overlap is handled separately: `[⟨a_s, b_s, b_c, a_c⟩]` retains a concurrent first-pass cut, rather than filtering away its observed overlap evidence.

## Using the library

Use `IMflc::default()` for the default threshold, or build a configured miner:

```rust
use inductive_miner::{EventLog, FilteringConfig, Miner};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let log = EventLog::default(); // Replace with a normalized lifecycle log.
    let tree = FilteringConfig::new(0.15)?.imflc().mine(&log)?;
    println!("{tree:#?}");
    Ok(())
}
```

For XES reading, see the [Quick start](../../README.md#quick-start). The default classifier reads `lifecycle:transition`. Preserve execution order and normalize the original input before mining.

## Guarantees and limitations

- **Soundness:** the result has sound process tree semantics under the appropriate lifecycle interpretation.
- **Fitness:** IMflc does not guarantee that all observed lifecycle traces fit the result. Filtering may remove events or optional behaviour, while repair inserts synthetic completions.
- **Rediscoverability:** the thesis's guarantee depends on its model-class and log assumptions, rather than arbitrary noisy lifecycle logs.

IMflc does not guarantee maximum precision or establish which rare behaviour is actually noise. Preserving overlap evidence during graph filtering can also limit which simplified cuts are available.

A zero threshold disables frequency-based graph removal and preserves observed empty traces. It still uses filtering splitters and consistency repair, so structural equality with IMlc's output is not guaranteed.

Lifecycle-aware interpretation is required to assess fitness and self-overlap. Ordinary named leaves do not encode explicit start/completion transitions, and atomic language checking cannot establish lifecycle fitness.

## Implementation locations

| Part | Source code |
|---|---|
| IMflc composition and builder | [`src/algorithms/imflc/mod.rs`](../../src/algorithms/imflc/mod.rs) |
| Lifecycle filtering base case | [`src/components/base_cases/single_non_atomic_activity_filtering.rs`](../../src/components/base_cases/single_non_atomic_activity_filtering.rs) |
| Lifecycle two-pass finder | [`src/components/cut_detections/non_atomic/filtering.rs`](../../src/components/cut_detections/non_atomic/filtering.rs) |
| Lifecycle interleaved detectors | [`src/components/cut_detections/non_atomic/interleaved_cut.rs`](../../src/components/cut_detections/non_atomic/interleaved_cut.rs) |
| DFG filtering | [`src/components/cut_detections/filtering.rs`](../../src/components/cut_detections/filtering.rs) |
| Consistency repair | [`src/components/log_splitting/consistency.rs`](../../src/components/log_splitting/consistency.rs) |
| Empty-trace filtering | [`src/components/fall_throughs/empty_traces_filtering.rs`](../../src/components/fall_throughs/empty_traces_filtering.rs) |
| IMflc tests | [`src/algorithms/imflc/tests.rs`](../../src/algorithms/imflc/tests.rs) |
