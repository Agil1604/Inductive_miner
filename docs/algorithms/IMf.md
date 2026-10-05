# Inductive Miner – infrequent (IMf)

IMf extends [IM](IM.md) to handle infrequent and deviating behaviour. It first attempts ordinary IM cut detection, then filters the directly-follows graph if no cut is found. Its base cases, splitters, and empty-trace handling can also discard observed behaviour.

This document describes IMf from section 6.2 of Sander Leemans' dissertation [*Robust Process Mining with Guarantees*](https://leemans.ch/publications/theses/phd.pdf).

## Input and output

The input is an `EventLog`. IMf uses activity labels, event order, and occurrence frequencies. Duplicate traces contribute separately to these frequencies.

The output is a `ProcessTree`.

`FilteringConfig` supplies a finite deviation threshold `f` in `[0, 1]`, defaulting to `0.2`. The same value controls graph filtering, the single-activity estimate, and empty-trace handling at every recursion level. It is not a global percentage of events to remove: each component applies its own rule.

## Base cases

`WithoutEmptyTraces<BaseCaseChain<EmptyLog, SingleActivityFiltering>>` checks the base cases only when no trace is empty. Empty traces are handled by the filtering fall-through.

### Empty log

As in IM, `EmptyLog` returns `τ` when the log contains no traces:

```text
L = []  →  τ
```

A log containing an empty trace is handled separately.

### Approximately one activity per trace

`SingleActivityFiltering` applies only when the log has exactly one distinct activity. Let `t` be the number of traces and `e` the total number of events. The component estimates:

```text
p = t / (e + t)
```

It returns the activity leaf when `|p − 0.5| ≤ f`, including equality. Exactly one event per trace gives `p = 0.5`; sufficiently small deviations can be simplified to the same leaf.

```text
L = [⟨a, a⟩]
p = 1 / 3
f = 0.2
Result: a
```

This removes the observed repetition. With `f = 0`, the estimate must equal `0.5`. The estimate concerns the aggregate event count, rather than independently checking that every trace has one event.

## Cut detection

Cut detection uses the DFG of the current sublog. See [architecture.md](../architecture.md) for the framework-level description and [IM.md](IM.md#cut-detection) for the ordinary detector rules.

`FilteringCutFinder` performs two passes:

1. Try the ordinary chain on the original graph.
2. If no cut is found, filter the graph and retry the same chain.

Both passes use this order:

```text
XorCut → SequenceCut → ConcurrentCut → LoopCut
```

An unfiltered cut takes precedence, even if the threshold is high. Graph filtering is attempted only when the entire first pass returns `None`.

### Relative-frequency filtering

For each source activity `a`, let `m(a)` be the highest outgoing edge count, including its end-activity count as an edge to an artificial sink. An edge is retained when:

```text
count(a → b) ≥ f × m(a)
```

End counts follow the same rule. Start counts share an artificial source and are compared with the maximum start count. Equality retains the relation.

For example, if `a → b` occurs 100 times and `a → c` occurs 5 times, with no larger outgoing count, `f = 0.2` retains the first edge and removes the second. An edge with count 20 is retained.

Filtering preserves activity vertices, their frequencies, and the empty-trace count. It changes the graph abstraction, not the original events. Both passes retain minimum self-distance witnesses from the original log. A matching splitter subsequently handles events that violate the selected cut.

A larger threshold removes more weak graph relations, allows a wider deviation in the single-activity base case, and requires more frequent empty traces to retain optionality. The resulting tree need not change monotonically with the threshold because different cuts and recursive decompositions may be selected.

## Log splitting

IMf splits the original sublog according to the selected cut, including when the cut was found before graph filtering.

### XOR: select the closest branch

`XorSplitFiltering` selects the partition containing the most events of each trace. Events belonging to other partitions are discarded, and the retained trace is assigned to the selected branch. Ties select the earliest partition.

```text
Trace: ⟨a, b, a⟩
Cut: ×({a}, {b})
Retained trace: ⟨a, a⟩ in the first branch
```

### Sequence: remove events that violate stage order

`SequenceSplitFiltering` selects successive trace segments for the ordered partitions and keeps only the current partition's events in each segment.

For each candidate segment, current-stage events reduce its cost, later-stage events increase its cost, and events from already processed stages are ignored. The earliest boundary attaining the minimum cost is selected. Events outside their selected stage segments are discarded; empty projections are retained.

Unlike ordinary projection, this can remove events whose position contradicts the sequence cut.

### Concurrency: ordinary projection

`ConcurrentSplit` is reused from IM without modification.

### Loop: allow missing body executions

`LoopSplitFiltering` divides traces into contiguous runs belonging to the body or redo partitions. If a trace starts in a redo partition, it adds an empty initial body trace. If it ends in a redo partition, it adds an empty final body trace.

```text
Trace: ⟨b, a, b⟩
Cut: loop({a}, {b})
L_body = [ε, ⟨a⟩, ε]
L_redo = [⟨b⟩, ⟨b⟩]
```

These empty traces let recursive empty-trace handling retain or filter the missing-body behaviour. This splitter does not simply delete every deviating loop event.

## Fall-throughs

IMf uses the same fall-through order as IM, replacing `EmptyTraces` with `EmptyTracesFiltering`:

| Component              | Condition and action                                                                                                                      |
| ---------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `EmptyTracesFiltering` | Remove empty traces and recursively mine the remainder. Retain an XOR branch with `τ` only when the empty-trace fraction is at least `f`. |
| `ActivityOncePerTrace` | Extract an activity occurring exactly once per trace as a concurrent branch.                                                              |
| `ActivityConcurrent`   | Extract an activity when its removal enables a cut in the remaining log.                                                                  |
| `StrictTauLoop`        | Split before subsequent start activities when preceded by an end activity.                                                                |
| `TauLoop`              | Split before subsequent start activities without the end-activity requirement.                                                            |
| `FlowerModel`          | Allow arbitrary nonempty sequences of observed activities.                                                                                |

Start and end activities refer to trace boundaries, not lifecycle transitions. See [IM.md](IM.md#fall-throughs) for the shared fall-through details.

For empty-trace count `k` and total trace count `t`, optionality is retained when `k ≥ f × t`. Equality retains it. A log consisting entirely of empty traces returns `τ` directly.

```text
L = [ε, ⟨a⟩, ⟨a⟩, ⟨a⟩, ⟨a⟩]
f = 0.2
Result: ×(τ, a)
```

With only one empty trace among ten traces and the same threshold, that empty behaviour is filtered out.

## Worked example

Consider a log with ten occurrences of `⟨a, b⟩`, one of `⟨b, a⟩`, and one of `⟨a, b, a⟩`, using `f = 0.3`.

1. Neither base case applies because the alphabet contains two activities.
2. Ordinary IM detection finds no cut: reverse edges prevent a sequence, while self-distance evidence prevents the concurrent cut.
3. The DFG has counts `a → b = 11` and `b → a = 2`. The end count of `b` is 11.
4. Filtering removes `b → a`, since `2 < 0.3 × 11`. Rare start/end boundaries are also removed, enabling `→({a}, {b})`.
5. The sequence splitter removes misplaced events. Recursive base cases and empty-trace filtering simplify the resulting sublogs.

The result is:

```text
→(a, b)
```

The reverse-order and repeated behaviour is treated as deviation rather than preserved in the discovered model.

## Using the library

Use `IMf::default()` for the default threshold, or build a configured miner:

```rust
use inductive_miner::{EventLog, FilteringConfig, Miner};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let log = EventLog::default(); // Replace with your event log.
    let miner = FilteringConfig::new(0.15)?.imf();
    let tree = miner.mine(&log)?;
    println!("{tree:#?}");
    Ok(())
}
```

For reading XES files, see the [Quick start](../../README.md#quick-start). IMf operates on atomic events and does not interpret lifecycle overlap; use IMflc for lifecycle-aware filtering.

## Guarantees and limitations

- **Soundness:** the algorithm returns a process tree that is sound under its standard semantics.
- **Fitness:** unlike IM, IMf does not guarantee that every observed trace fits the result. Filtering intentionally allows observed behaviour to be discarded.
- **Rediscoverability:** the dissertation establishes rediscoverability under its source-model and log assumptions, rather than for arbitrary noisy logs.

IMf does not guarantee maximum precision or identify which events are truly noise. Rare behaviour can be valid, and frequent behaviour can be erroneous. The threshold controls the algorithm's filtering rules rather than expressing confidence that an event is incorrect.

A zero threshold disables frequency-based graph removal and retains observed empty traces. It still uses IMf's base-case and splitting rules; it should not be treated as a promise of structural equality with the tree returned by IM.

## Implementation locations

| Part                                       | Source code                                                                                                              |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------ |
| IMf composition and configuration builders | [`src/algorithms/imf/mod.rs`](../../src/algorithms/imf/mod.rs)                                                           |
| Shared filtering configuration             | [`src/components/filtering.rs`](../../src/components/filtering.rs)                                                       |
| Filtering base case                        | [`src/components/base_cases/single_activity_filtering.rs`](../../src/components/base_cases/single_activity_filtering.rs) |
| DFG filtering and two-pass detection       | [`src/components/cut_detections/filtering.rs`](../../src/components/cut_detections/filtering.rs)                         |
| Log splitters                              | [`src/components/log_splitting`](../../src/components/log_splitting)                                                     |
| Empty-trace filtering                      | [`src/components/fall_throughs/empty_traces_filtering.rs`](../../src/components/fall_throughs/empty_traces_filtering.rs) |
| IMf tests                                  | [`src/algorithms/imf/tests.rs`](../../src/algorithms/imf/tests.rs)                                                       |
