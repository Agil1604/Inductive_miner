# Inductive Miner – all operators (IMa)

IMa extends [IM](IM.md) with interleaving and inclusive choice. It also refines sequence detection so that related optional stages can remain in one subtree instead of being treated as independently optional.

This document describes IMa from section 6.4.2 of Sander Leemans' dissertation [*Robust Process Mining with Guarantees*](https://leemans.ch/publications/theses/phd.pdf).

## Input and output

The input is an `EventLog`. IMa uses activity labels, event order, and the sets of activities present in each trace. 

The output is a `ProcessTree` supporting all six operators.

## Base cases

IMa reuses `BaseCaseChain<EmptyLog, SingleActivity>` from IM:

- `EmptyLog` returns `τ` when the log contains no traces.
- `SingleActivity` returns a leaf when every trace contains exactly one event of the same activity.

A log containing an empty trace is handled by `EmptyTraces`, preserving optionality. See [IM.md](IM.md#base-cases) for examples.

## Cut detection

`CutFinder` prepares the DFG and shares the context across this chain:

```text
XorCut → SequenceCutStrict → InterleavedCut → CooCut → LoopCut
```

The first applicable cut is selected. `CooCut` can return either a concurrent or an inclusive-choice cut, so a separate `ConcurrentCut` is not a top-level stage in this chain. It is used internally by `CooCut`.

The ordinary XOR and loop detectors are reused. See [IM.md](IM.md#cut-detection) for their rules and [architecture.md](../architecture.md) for context preparation and composition.

### Strict sequence: preserve the scope of optional behaviour

`SequenceCutStrict` starts with the ordered partition produced by `SequenceCut`. It identifies skippable parts: an edge can bypass them, a trace can start after them, or a trace can end before them. Incoming and outgoing edge bounds then determine neighbouring parts that must be merged with the skippable part.

These merges preserve a larger optional scope for recursive discovery.

### Interleaving: execute complete blocks in either order

`InterleavedCut` checks a directly-follows footprint: across different parts, a directed edge must exist exactly when its source is an observed end activity and its target an observed start activity. Violations and minimum self-distance witnesses merge activities into the same part.

A further pass scans the original traces. If execution returns to a previously visited part after visiting another part, the detector merges parts and rescans. Every final part must occupy at most one contiguous block in each trace.

Interleaved detection precedes `CooCut`. When several operators can explain atomic traces, that priority affects the selected representation.

### Coo: distinguish concurrency and inclusive choice

`CooCut` implements the concurrent-optional-or relations from section 5.6 of the thesis. it begins with a partition from `ConcurrentCut`, using bidirectional DFG edges, start/end boundaries, and self-distance witnesses. If no such partition exists, `CooCut` returns `None`.

It then represents each trace by the set of partitions present in that trace. This presence language ignores event order and duplicate presence patterns. Three relations guide merging:

- **Implication:** whenever one part occurs, another occurs too.
- **Optionality:** for every pattern containing a part, the corresponding pattern without that part also occurs.
- **Interchangeability:** keeping all other parts unchanged, patterns exist with the first part alone, the second alone, and both together.

While more than two parts remain, related pairs are merged according to the concurrency, concurrent optionality, and interchangeability rules. Presence patterns are rebuilt after each merge. Concurrent optionality additionally uses implication and DFG evidence involving other parts.

When merging stops, a remaining interchangeable pair selects `InclusiveChoice`; otherwise the detector returns `Concurrent`.

## Log splitting

`ExtendedLogSplitter` adds interleaved and inclusive-choice dispatch to the ordinary IM splitters. IMa does not filter deviating events.

For inclusive choice, an absent branch expresses selection of another branch, not a silent execution inside the absent branch. Discarding empty projections keeps that optionality at the inclusive-choice operator.

All splitters preserve the order and metadata of retained events and share the original activity interner.

## Fall-throughs

IMa reuses IM's fall-through composition. See [IM.md](IM.md#fall-throughs) for the individual rules.

## Worked example

Consider:

```text
L = [⟨a, b, c⟩, ⟨c, a, b⟩]
```

1. Neither base case applies.
2. The DFG is connected, so no XOR cut exists.
3. The observed orders do not admit a sequence cut between `{a, b}` and `{c}`.
4. Interleaved detection groups `a` and `b`, while keeping `c` separate.
5. Projection produces `[⟨a, b⟩, ⟨a, b⟩]` and `[⟨c⟩, ⟨c⟩]`.
6. Recursion discovers `→(a, b)` and leaf `c`.

The result is:

```text
↔(→(a, b), c)
```

Both observed block orders are allowed, while `⟨a, c, b⟩` is excluded.

## Using the library

```rust
use inductive_miner::{algorithms::IMa, EventLog, Miner};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let log = EventLog::default(); // Replace with your event log.
    let tree = IMa::default().mine(&log)?;
    println!("{tree:#?}");
    Ok(())
}
```

For reading XES input, see the [Quick start](../../README.md#quick-start).
IMa uses atomic activity order; it does not interpret lifecycle overlap or sort
by timestamps. Use IMfa when filtering is required.

## Guarantees and limitations

- **Soundness:** the result is a process tree with sound standard semantics.
- **Fitness:** IMa preserves observed atomic traces rather than filtering them.
- **Rediscoverability:** the thesis establishes guarantees under its specified
  source-model class and log assumptions. Additional operators do not imply
  recovery of every arbitrary process from incomplete observations.

IMa does not guarantee maximum precision. Its strict sequence and extended
operators can represent behaviour more precisely than basic IM in suitable
cases, but loops and fall-throughs can still introduce unobserved behaviour.
The cut priority also determines which representation is chosen when several
operators explain the same observations.

## Implementation locations

| Part                      | Source code                                                                                                              |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| IMa composition           | [`src/algorithms/ima/mod.rs`](../../src/algorithms/ima/mod.rs)                                                           |
| Strict sequence detector  | [`src/components/cut_detections/sequence_cut_strict.rs`](../../src/components/cut_detections/sequence_cut_strict.rs)     |
| Interleaved detector      | [`src/components/cut_detections/interleaved_cut.rs`](../../src/components/cut_detections/interleaved_cut.rs)             |
| Coo detector              | [`src/components/cut_detections/coo_cut.rs`](../../src/components/cut_detections/coo_cut.rs)                             |
| Interleaved splitter      | [`src/components/log_splitting/interleaved_split.rs`](../../src/components/log_splitting/interleaved_split.rs)           |
| Inclusive-choice splitter | [`src/components/log_splitting/inclusive_choice_split.rs`](../../src/components/log_splitting/inclusive_choice_split.rs) |
| IMa tests                 | [`src/algorithms/ima/tests.rs`](../../src/algorithms/ima/tests.rs)                                                       |
