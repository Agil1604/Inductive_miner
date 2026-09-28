# Inductive Miner (IM)

Inductive Miner is a process discovery algorithm that builds a process tree from an event log. It recursively divides the log into simpler sublogs and combines their discovered models using process tree operators.

This document describes the basic IM algorithm from §6.1 of Sander Leemans' dissertation [*Robust Process Mining with Guarantees*](https://leemans.ch/publications/theses/phd.pdf).

## Input and output

The input is an `EventLog`. IM uses the activity labels and their order. Duplicate traces and repeated events are retained; an empty trace records an execution without observable events.

The output is a `ProcessTree`.


## Base cases

`BaseCaseChain<EmptyLog, SingleActivity>` checks two cases in order.

### Empty log

`EmptyLog` returns `τ` when the log contains no traces.

```text
L = []  →  τ
```

An empty log differs from a log containing an empty trace. `L = [ε]` records an execution, so `EmptyLog` does not apply to it.

### One activity per trace

`SingleActivity` returns an activity leaf `a` when every trace contains exactly one event and all these events have the same activity label.

```text
L = [⟨a⟩, ⟨a⟩]  →  a
```

This case does not apply to `[⟨a, a⟩]`: repetition requires a loop. It also does not apply to `[ε, ⟨a⟩]`, because the model must account for the optional execution
of `a`.

## Cut detection
Cut detection uses a directly-follows graph (DFG) of the current sublog. See [architecture.md](../architecture.md) for the framework-level description.

A cut specifies an operator and divides the activity alphabet into nonempty, disjoint parts `Σ₁, …, Σₙ`. Every activity in the current log must belong to exactly one part. For a sequence, partition order determines execution order; for a loop, the first partition is the body.

`CutChain` selects the **first** cut found in this order:

```text
XorCut → SequenceCut → ConcurrentCut → LoopCut
```

The detectors are tried in order. If their merging rules leave only one part, they return `None`, allowing the chain to try the next detector.

### XOR: independent alternatives

`XorCut` finds connected components of the DFG while treating edges as
undirected. Different components have no directly-follows connections between them and can represent alternative branches.

```text
L = [⟨a, b⟩, ⟨c, d⟩]
Cut: ×({a, b}, {c, d})
```

`XorSplit` assigns each complete trace to its corresponding branch. For a valid XOR cut, all events in a nonempty trace belong to the same partition.

### Sequence: ordered stages

`SequenceCut` uses reachability in the DFG. Activities are merged when they are mutually reachable or when neither is reachable from the other. The remaining groups are ordered by the direction of reachability.

```text
L = [⟨a, b, c⟩, ⟨a, c⟩]
Cut: →({a}, {b}, {c})
```

`SequenceSplit` projects every trace onto every partition. A projection removes events belonging to other partitions while preserving the order of the retained events. When a stage is absent from a trace, its projection is empty:

```text
L₁ = [⟨a⟩, ⟨a⟩]
L₂ = [⟨b⟩, ε]
L₃ = [⟨c⟩, ⟨c⟩]
```

Empty projections tell the recursive miner that a stage may be skipped.

### Concurrency: interwoven branches

`ConcurrentCut` starts with separate activities and merges pairs that lack DFG edges in both directions. It also uses minimum self-distance witnesses to prevent certain false concurrency cuts.

It also uses minimum self-distance witnesses to prevent false concurrency cuts.

Partitions without a start or end activity are then merged: each remaining parallel branch must contain both boundaries.

```text
L = [⟨a, b⟩, ⟨b, a⟩]
Cut: ∧({a}, {b})
```

`ConcurrentSplit` projects every trace onto every branch and retains empty projections. The tree operator represents the possible interweaving, while each branch retains its own event order.

### Loop: a body and redo branches

`LoopCut` places all start and end activities in the body. The remaining activities form connected redo candidates. Candidates are absorbed into the body if they have forbidden boundary connections or connect to some but not all required start or end activities.

```text
L = [⟨a, b, a⟩, ⟨a⟩]
Cut: loop({a}, {b})
```

`LoopSplit` extracts maximal contiguous runs belonging to one partition. Multiple runs from an original trace become separate traces in the sublogs:

```text
L_body = [⟨a⟩, ⟨a⟩, ⟨a⟩]
L_redo = [⟨b⟩]
Result: loop(a, b)
```

## Fall-throughs

When no regular cut is found, IM tries additional decompositions in the following order. Their recursive calls use the current miner, preserving its component configuration.

| Component | Condition and action |
|---|---|
| `EmptyTraces` | If empty traces exist, return `×(τ, IM(nonempty traces))`. |
| `ActivityOncePerTrace` | Find an activity occurring exactly once in every trace; return its leaf concurrently with the recursively mined remaining events. |
| `ActivityConcurrent` | Try removing one activity. If the remaining log admits a cut, recursively mine the removed activity and the remaining events as concurrent branches. |
| `StrictTauLoop` | Split before a subsequent occurrence of a start activity only when the preceding event is an end activity. |
| `TauLoop` | Split before subsequent occurrences of start activities without requiring an end activity immediately before them. |
| `FlowerModel` | Return a model allowing arbitrary nonempty sequences of observed activities. |

Here, start and end activities refer to trace boundaries in the current sublog, not to the lifecycle transitions `Start` and `Complete`.

Both tau-loop components apply only when at least one trace is actually split.
The resulting segments are recursively mined as the loop body, with `τ` as the redo branch.

For `[⟨a, a⟩]`, regular cuts cannot apply because the alphabet contains one activity. `StrictTauLoop` splits the trace into `[⟨a⟩, ⟨a⟩]`, producing `loop(a, τ)`.

For alphabet `{a, b}`, the flower model is `loop(×(a, b), τ)`. This fallback preserves observed behaviour by allowing many additional traces. For an empty alphabet, the component returns `τ`.

## Worked example

Consider the log:

```text
L = [⟨a, b, d⟩, ⟨a, c, d⟩]
```

1. Neither base case applies because the traces contain multiple events.
2. No XOR cut exists because the entire DFG is connected.
3. A sequence cut `→({a}, {b, c}, {d})` is found.
4. Projection produces `[⟨a⟩, ⟨a⟩]`, `[⟨b⟩, ⟨c⟩]`, and `[⟨d⟩, ⟨d⟩]`.
5. The first and third sublogs produce leaves `a` and `d`.
6. The middle sublog admits an XOR cut, whose children produce leaves `b` and `c`.

The result is:

```text
→(a, ×(b, c), d)
```

The model represents a choice between `b` and `c` after `a`.

## Using the library

For a usage example, see the [Quick start](../../README.md#quick-start). Basic IM operates on atomic event logs: it uses activity labels and event order, and does not consult timestamps or lifecycle transitions.

## Guarantees and limitations

The dissertation establishes the following properties for IM:

- **Soundness:** under standard process tree semantics, the discovered model has no deadlocks and permits proper completion.
- **Fitness:** the model admits all traces in the original atomic event log.
- **Rediscoverability:** under the specified log completeness assumptions and restrictions on the source model class, IM recovers its behaviour. This does not promise recovery of an arbitrary process from any incomplete log.

IM does not guarantee maximum precision: the result may allow behaviour absent from the input. For example, a loop generalizes finitely many observed repetitions to arbitrarily many repetitions, while a flower model allows arbitrary sequences of activities.

These guarantees concern the algorithm under its assumptions. They do not automatically apply to every custom composition of `InductiveMiner` components. Implementation tests cover examples and bounded collections of traces, but do not replace the dissertation's proofs.

Basic IM does not filter noise, use event durations, or account for overlapping lifecycle executions. Other variants, including IMf and lifecycle-aware miners, address these tasks.

## Implementation locations

| Part | Source code |
|---|---|
| IM composition | [`src/algorithms/im/mod.rs`](../../src/algorithms/im/mod.rs) |
| Recursive miner | [`src/framework/miner.rs`](../../src/framework/miner.rs) |
| Stage contracts | [`src/framework/traits.rs`](../../src/framework/traits.rs) |
| Base cases | [`src/components/base_cases`](../../src/components/base_cases) |
| Cuts and shared context | [`src/components/cut_detections`](../../src/components/cut_detections) |
| Log splitting | [`src/components/log_splitting`](../../src/components/log_splitting) |
| Fall-throughs | [`src/components/fall_throughs`](../../src/components/fall_throughs) |
| IM tests | [`src/algorithms/im/tests.rs`](../../src/algorithms/im/tests.rs) |
