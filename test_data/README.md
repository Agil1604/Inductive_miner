# Test data

The `xes_examples/` directory contains event logs for testing XES parsing and process discovery.

| File | Traces | Events | Contents and testing purpose |
|---|---:|---:|---|
| [five_traces_with_repetitions.xes](xes_examples/five_traces_with_repetitions.xes) | 5 | 18 (3–5 per trace) | Activity-only traces with repeated activities and a duplicate trace. Tests event order, multiplicities, DFG self-loops, and IM discovery. Case IDs, timestamps, and lifecycle annotations are absent. |
| [nested_attributes.xes](xes_examples/nested_attributes.xes) | 2 | 3 | Named cases containing activities `A`, `B`, and `C`, plus nested containers and typed metadata. Tests that metadata does not become events or override the configured timestamp attribute. Custom metadata is not retained by the current reader. |
| [three_cases_with_lifecycle.xes](xes_examples/three_cases_with_lifecycle.xes) | 3 | 6 (2 per trace) | Cases `case-1`, `case-2`, and `case-3`, each with a start/complete pair and UTC timestamps. Tests case IDs, lifecycle parsing, timestamps, and atomic projection. |
| [pdc2025_1000_traces.xes](xes_examples/pdc2025_1000_traces.xes) | 1,000 | 16,326 | PDC 2025 log with named cases and activity-only events. Tests parsing a larger log. |
