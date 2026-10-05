# Inductive Miner – infrequent, all operators (IMfa)

IMfa combines [IMa](IMa.md)'s six process tree operators with [IMf](IMf.md)'s handling of infrequent and deviating behaviour. It first attempts IMa cut detection, then retries selected detectors on a filtered DFG. A filtering interleaved splitter removes events that violate whole-block execution.

This document describes IMfa from section 6.4.3 of Sander Leemans' dissertation [*Robust Process Mining with Guarantees*](https://leemans.ch/publications/theses/phd.pdf).

## Input and output

The input is an `EventLog`. IMfa uses atomic activity labels, event order, frequencies, and activity-presence patterns. Duplicate traces contribute separately to frequency counts.

The output is a `ProcessTree`.

`FilteringConfig` supplies a finite threshold `f` in `[0, 1]`, defaulting to `0.2`. It controls DFG filtering, the single-activity estimate, and empty-trace handling throughout recursion. It does not specify a global percentage of events to remove.

## Base cases

IMfa reuses IMf's guarded filtering base cases:

```text
WithoutEmptyTraces<BaseCaseChain<EmptyLog, SingleActivityFiltering>>
```

`EmptyLog` returns `τ` for a log without traces. For a log with exactly one activity, `SingleActivityFiltering` returns its leaf when:

```text
|t / (e + t) − 0.5| ≤ f
```

Here, `t` is the number of traces and `e` the total event count. This can simplify observed repetitions to a single activity. Empty traces are deferred to the fall-through. See [IMf.md](IMf.md#base-cases) for examples.

The prose of section 6.4.3 explicitly reuses IMf's base cases and fall-throughs, while the summary pseudocode names IM's versions. This implementation follows the prose and uses filtering in both stages.

## Cut detection

`TwoPassFilteringCutFinder` allows different chains for the original and filtered graphs. An unfiltered cut is returned immediately; the second pass runs only when the entire first chain fails.

### First pass: ordinary IMa detection

```text
XorCut → SequenceCutStrict → InterleavedCut → CooCut → LoopCut
```

This pass preserves IMa's priority and can detect all six operators. It uses the original DFG, trace-block checks, self-distance witnesses, and presence evidence. See [IMa.md](IMa.md#cut-detection) for the detector rules.

### Second pass: filtered detection

```text
XorCut → SequenceCutStrict → InterleavedCutFiltering → LoopCut
```

The DFG is filtered using IMf's relative outgoing-frequency rule. An edge is retained when its count is at least `f` times the maximum outgoing count of its source. Start and end boundaries are filtered with their corresponding source rules. Equality retains the relation.

The second pass omits `CooCut`, so it does not retry concurrent or inclusive-choice detection on the filtered graph. Those operators can still be found by the first pass or within recursively mined sublogs.

Vertices remain present after graph filtering, and minimum self-distance witnesses still come from the original log. The filtering splitter subsequently handles the original events. See [IMf.md](IMf.md#relative-frequency-filtering) for the frequency rule.

### Filtered interleaving

`InterleavedCutFiltering` applies the end-to-start DFG footprint and minimum self-distance witness checks, but omits the original-trace block-merging pass. A trace may therefore leave a partition and later return to it. The matching splitter removes deviations instead of merging every such partition together.

This is an implementation interpretation of the filtered interleaving step: retaining the ordinary trace-merging pass would eliminate the candidate before the filtering splitter could repair the violation.

## Log splitting

IMfa uses filtering splitters even when its first detection pass finds the cut.

The four basic splitters follow [IMf's rules](IMf.md#log-splitting). Inclusive-choice splitting follows [IMa's rule](IMa.md#log-splitting): project onto each selected part and discard empty projections. Concurrent splitting uses ordinary projection.

### Divide-and-conquer interleaved splitting

For each input trace, `InterleavedSplitFiltering` chooses a partition and an interval maximizing:

```text
score = matching events − other events inside the interval
```

It then:

1. Keeps only the selected partition's events inside that interval.
2. Discards the selected partition's events outside the interval.
3. Removes those events from the left and right outer sequences.
4. Recursively processes the outer sequences for other partitions.

Equal scores retain the first interval encountered in the left-to-right scan. The output contains one trace per input case for each partition, including empty traces. Retained events preserve their order and metadata, and all sublogs share the input interner.

Following the thesis's `splitTrace` assignment, if the same partition is selected in both outer recursions, the right recursion's result replaces the earlier one. The implementation does not concatenate these results.

## Fall-throughs

IMfa uses the filtering fall-through composition shared with IMf:

```text
EmptyTracesFiltering
→ ActivityOncePerTrace
→ ActivityConcurrent
→ StrictTauLoop
→ TauLoop
→ FlowerModel
```

`EmptyTracesFiltering` retains optionality only when the empty-trace fraction is at least `f`. The remaining components use the current IMfa miner for recursion, so their sublogs can still produce all six operators.

See [IMf.md](IMf.md#fall-throughs) for empty-trace filtering and [IM.md](IM.md#fall-throughs) for the shared strategies.

## Worked example

Consider a selected interleaved cut with partitions `{a}`, `{b}`, and `{c}`, and the trace:

```text
⟨a, a, b, a, a, b, b, b, b, c⟩
```

The trace does not execute each part as one block: `a` appears on both sides of the first `b`. Ordinary interleaved splitting cannot repair this violation.

The filtering splitter selects the long final `b` interval, containing four `b` events. It removes the earlier `b` from the left outer sequence. Recursion then retains the four `a` events and the final `c`:

```text
L_a = [⟨a, a, a, a⟩]
L_b = [⟨b, b, b, b⟩]
L_c = [⟨c⟩]
```

One `b` event is discarded. Recursion mines each retained branch; the interleaved operator combines them as complete blocks. This example illustrates splitting for a supplied cut, rather than asserting that the full miner necessarily selects that cut for a log containing only this trace.

## Using the library

Use `IMfa::default()` for the default threshold, or build a configured miner:

```rust
use inductive_miner::{EventLog, FilteringConfig, Miner};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let log = EventLog::default(); // Replace with your event log.
    let tree = FilteringConfig::new(0.15)?.imfa().mine(&log)?;
    println!("{tree:#?}");
    Ok(())
}
```

For reading XES input, see the [Quick start](../../README.md#quick-start). IMfa does not interpret lifecycle overlap or sort events by timestamp. IMflc provides lifecycle-aware frequency filtering with a different detector composition.

## Guarantees and limitations

- **Soundness:** the result is a process tree with sound standard semantics.
- **Fitness:** filtering can remove observed events or empty behaviour, so IMfa does not guarantee that all input traces fit the result.
- **Rediscoverability:** the thesis's guarantees depend on its model-class and log assumptions, not arbitrary noisy or incomplete observations.

IMfa does not guarantee maximum precision or establish which events are actually noise. Valid rare behaviour can be discarded. The interleaved splitter's local interval selection is a heuristic, not a general proof of globally optimal trace repair.

A zero threshold disables frequency-based graph removal and preserves observed empty traces, but IMfa still uses filtering base cases and splitters. Structural equality with IMa's output is not guaranteed.

## Implementation locations

| Part                           | Source code                                                                                                                        |
| ------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------- |
| IMfa composition and builders  | [`src/algorithms/imfa/mod.rs`](../../src/algorithms/imfa/mod.rs)                                                                   |
| Shared filtering configuration | [`src/components/filtering.rs`](../../src/components/filtering.rs)                                                                 |
| Two-pass cut adapter           | [`src/components/cut_detections/filtering.rs`](../../src/components/cut_detections/filtering.rs)                                   |
| Interleaved detectors          | [`src/components/cut_detections/interleaved_cut.rs`](../../src/components/cut_detections/interleaved_cut.rs)                       |
| Filtering interleaved splitter | [`src/components/log_splitting/interleaved_split_filtering.rs`](../../src/components/log_splitting/interleaved_split_filtering.rs) |
| IMfa tests                     | [`src/algorithms/imfa/tests.rs`](../../src/algorithms/imfa/tests.rs)                                                               |
