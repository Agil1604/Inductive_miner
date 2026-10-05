# Inductive Miner – life cycle (IMlc)

IMlc extends [IM](IM.md) to distinguish activity executions from individual events. A start and a completion can describe one execution, and overlapping executions provide direct evidence of concurrency. IMlc uses a non-atomic DFG and a concurrency graph to discover this behaviour.

This document describes IMlc from section 6.5.2 of Sander Leemans' dissertation [*Robust Process Mining with Guarantees*](https://leemans.ch/publications/theses/phd.pdf). Sections 5.7.3–5.7.4 introduce the graph abstractions; section 6.5.4 discusses practical lifecycle handling.

## Input and output

The input is an `EventLog` with lifecycle transitions. In the examples, `a_s` denotes the start of activity `a`, and `a_c` its completion.

```text
⟨a_s, b_s, b_c, a_c⟩
```

This trace describes one execution of `a` overlapping one execution of `b`. It does not describe two atomic executions of each activity.

The output is a `ProcessTree` supporting XOR, sequence, concurrency, interleaving, and loop. IMlc does not detect inclusive choice. A leaf retains the activity name rather than becoming separate start and completion leaves; consumers must interpret it as an execution when reasoning about lifecycle behaviour.

Events are processed in their stored trace order. Timestamps are not consulted or sorted. Explicit start/completion instance identifiers are not required: the graph builders track activity execution counts.

### Lifecycle assumptions

The thesis's formal algorithm assumes consistent traces: each start has a corresponding later completion of the same activity, and vice versa. Its implementation notes allow unmatched completions as atomic executions.

The library follows these conventions:

| Input                                     | Interpretation                                                   |
| ----------------------------------------- | ---------------------------------------------------------------- |
| `Start` followed by a matching `Complete` | One lifecycle execution.                                         |
| Unmatched `Complete`                      | An instantaneous atomic execution.                               |
| `Unknown`                                 | An instantaneous atomic execution; this is a library convention. |
| `Enqueue` or `Other`                      | Ignored by lifecycle graph construction.                         |
| Unmatched `Start`                         | Not automatically repaired by IMlc.                              |

Normalize input before mining: remove ignored transitions and resolve unmatched starts. The graph builders ignore unsupported transitions, but the shared splitters and framework still receive the original log. Ignoring transitions in graph preparation alone is not a complete preprocessing pipeline.

Calling `EventLog::atomic()` is a different operation: it retains completion and unknown events but removes starts, losing the overlap evidence IMlc needs.

## Base cases

IMlc composes:

```text
WithoutEmptyTraces<BaseCaseChain<EmptyLog, SingleNonAtomicActivity>>
```

### Empty log

`EmptyLog` returns `τ` when the log contains no traces. A log containing an empty trace is deferred to `EmptyTraces`.

### One execution per trace

`SingleNonAtomicActivity` returns a leaf when every trace contains exactly one execution of the same activity. It accepts a start/completion pair, a lone completion, or a lone unknown event.

```text
L = [⟨a_s, a_c⟩, ⟨a_s, a_c⟩]
Result: a
```

It rejects unmatched starts, mismatched activity pairs, repeated executions, and traces without an execution. Enqueue and other transitions are ignored by this component.

## Cut detection

`NonAtomicCutFinder` prepares one `NonAtomicCutContext`, containing the activity alphabet, original log, non-atomic DFG, and concurrency graph. See [architecture.md](../architecture.md) for the composition contracts.

The detector chain is:

```text
NonAtomicDfgCut<XorCut>
→ NonAtomicDfgCut<SequenceCut>
→ NonAtomicConcurrentCut
→ NonAtomicInterleavedCut
→ NonAtomicLoopCut
```

The first applicable cut is selected. Ordinary sequence detection is used, rather than IMa's `SequenceCutStrict`.

### Non-atomic DFG

`DirectlyFollowsGraph::from_non_atomic_log` records completion-to-start succession. A completion of `a` directly precedes a start of `b` when the intervening events contain no full execution of an activity.

Unlike an ordinary DFG, it does not add an edge for every pair of adjacent lifecycle events. Activity frequencies count executions rather than counting the start and completion separately. Atomic events are represented internally by instantaneous start/completion pairs.

The current builder records starts before the first completion as start activities and completions after the final start as end activities. Several activities can therefore have start/end evidence within one trace.

### Concurrency graph

`ConcurrencyGraph` records activity pairs whose executions overlap. Edges are undirected; self-edges represent simultaneous executions of the same activity. It also records maximum simultaneous execution counts for the concurrent flower fallback.

```text
Trace: ⟨a_s, b_s, b_c, a_c⟩
Overlap: a ∥ b
```

This evidence comes from lifecycle event order, even when no timestamp is recorded.

### XOR and sequence

The existing `XorCut` and `SequenceCut` strategies are reused through `NonAtomicDfgCut`. Its prepared graph supplements the completion-to-start DFG with symmetric edges for observed overlap. This prevents overlapping activities from being separated into alternative branches or sequential stages.

The supplemented graph is cached in the context and shared by both adapters. The lifecycle-specific detectors use the original non-atomic DFG and concurrency graph separately.

### Concurrency

`NonAtomicConcurrentCut` permits activities in different parts when either:

- both directly-follows directions exist; or
- their executions have an observed overlap.

Pairs lacking both forms of evidence are merged. Parts without start or end evidence are then merged. Atomic minimum self-distance witnesses are not used.

### Interleaving

`NonAtomicInterleavedCut` checks the end-to-start DFG footprint and then scans the original lifecycle events. If a trace leaves a partition and later returns to it, partitions are merged until every part occupies a single block.

This prevents overlapping executions from being assigned to different interleaved branches. Unlike ordinary `InterleavedCut`, it omits atomic self-distance witness merging.

### Loop

`NonAtomicLoopCut` places start and end activities in the body. Remaining activities form redo candidates connected by directly-follows edges or overlap.

Candidates violating boundary requirements are merged into the body. Candidates overlapping the body are also absorbed, with overlap checks repeated as the body grows. Overlapping activities must not be separated into different loop parts, where splitting could disconnect a start from its completion.

## Log splitting

IMlc reuses unfiltered splitters.

Projection retains both start and completion events belonging to the selected activities. Loop splitting extracts contiguous runs. The lifecycle cut rules are intended to ensure that valid splits preserve execution consistency rather than repairing it afterwards.

For example:

```text
Trace: ⟨a_s, b_s, b_c, a_c⟩
Cut: ∧({a}, {b})
L_a = [⟨a_s, a_c⟩]
L_b = [⟨b_s, b_c⟩]
```

All retained events preserve order, timestamps, lifecycle values, and case IDs. Sublogs share the original interner. The extended splitter adapter can also dispatch inclusive choice, but IMlc's detector chain never produces that operator.

## Fall-throughs

IMlc uses this order:

| Component                       | Condition and action                                                                                           |
| ------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| `EmptyTraces`                   | Retain empty behaviour as `×(τ, IMlc(nonempty traces))`.                                                       |
| `NonAtomicActivityOncePerTrace` | Extract an activity with exactly one lifecycle execution per trace as a concurrent branch.                     |
| `ActivityConcurrent`            | Reuse activity extraction when removing an activity enables a cut under the configured lifecycle finder.       |
| `StrictNonAtomicTauLoop`        | Split before a start activity only after all executions close and a preceding end-activity completion exists.  |
| `NonAtomicTauLoop`              | Split before a start activity only after all executions close, without the preceding end-activity requirement. |
| `ConcurrentFlowerModel`         | Create enough parallel optional repeatable activity branches to cover maximum observed self-concurrency.       |

Both tau-loop components require an actual split. Their closed-execution condition prevents a start and completion from entering different subtraces. For example, an enclosing execution of `a` prevents splitting between repeated `b` executions:

```text
⟨a_s, b_s, b_c, b_s, b_c, a_c⟩
```

### Concurrent flower model

An ordinary flower model cannot represent two overlapping executions of the same activity. The lifecycle fallback creates one optional repeatable branch per maximum simultaneous execution, then combines these branches concurrently.

```text
Trace: ⟨a_s, b_s, a_s, b_c, a_c, a_c⟩
Maximum concurrency: a = 2, b = 1
Fallback: ∧(loop(τ, a), loop(τ, a), loop(τ, b))
```

Here, `loop(τ, a)` allows zero or more executions of `a`. Repeated named leaves represent independent execution slots. This fallback allows additional behaviour beyond the observed trace.

## Worked example

Consider the thesis's nested-overlap example:

```text
L = [⟨a_s, b_s, b_c, a_c⟩]
```

1. No base case applies because two activities are present.
2. The concurrency graph records overlap between `a` and `b`.
3. Overlap protection prevents XOR and sequence cuts from separating them.
4. `NonAtomicConcurrentCut` finds `∧({a}, {b})`.
5. Splitting produces `[⟨a_s, a_c⟩]` and `[⟨b_s, b_c⟩]`.
6. Each sublog matches `SingleNonAtomicActivity`.

The result is:

```text
∧(a, b)
```

Under lifecycle semantics, the activities can overlap. Removing starts before discovery would leave `⟨b_c, a_c⟩` and lose that information.

## Using the library

```rust
use inductive_miner::{algorithms::IMlc, EventLog, Miner};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let log = EventLog::default(); // Replace with a normalized lifecycle log.
    let tree = IMlc::default().mine(&log)?;
    println!("{tree:#?}");
    Ok(())
}
```

For XES reading, see the [Quick start](../../README.md#quick-start). The default classifier reads `lifecycle:transition`. Preserve the intended event order and prepare consistent input before mining.

## Guarantees and limitations

- **Soundness:** the discovered tree has sound semantics when interpreted with the appropriate lifecycle execution model.
- **Fitness:** the thesis establishes lifecycle fitness for consistent input and its prescribed lifecycle components.
- **Rediscoverability:** the guarantee depends on the specified source-model class and completeness of the DFG and concurrency evidence.

These are reference-algorithm guarantees under their assumptions. The implementation's graph conventions and overlap adapter are described above; structural validation and example tests do not independently establish every theorem for those implementation choices.

The output has ordinary named leaves without an explicit lifecycle marker. Atomic language checking cannot establish lifecycle fitness: it does not model starts, completions, or simultaneous executions of one activity.

IMlc does not filter noise or guarantee maximum precision. Use IMflc for frequency filtering and IMclc for probabilistic handling of incomplete observations. Input consistency validation and normalization are separate concerns; IMlc does not automatically repair unmatched starts.

## Implementation locations

| Part                     | Source code                                                                                                                |
| ------------------------ | -------------------------------------------------------------------------------------------------------------------------- |
| IMlc composition         | [`src/algorithms/imlc/mod.rs`](../../src/algorithms/imlc/mod.rs)                                                           |
| Lifecycle base case      | [`src/components/base_cases/single_non_atomic_activity.rs`](../../src/components/base_cases/single_non_atomic_activity.rs) |
| Non-atomic DFG builder   | [`src/model/directly_follows_graph.rs`](../../src/model/directly_follows_graph.rs)                                         |
| Concurrency graph        | [`src/model/concurrency_graph.rs`](../../src/model/concurrency_graph.rs)                                                   |
| Lifecycle cut components | [`src/components/cut_detections/non_atomic`](../../src/components/cut_detections/non_atomic)                               |
| Lifecycle fall-throughs  | [`src/components/fall_throughs/non_atomic`](../../src/components/fall_throughs/non_atomic)                                 |
| IMlc tests               | [`src/algorithms/imlc/tests.rs`](../../src/algorithms/imlc/tests.rs)                                                       |
