# Inductive Miner

`inductive-miner` is a Rust framework implementing the Inductive Miner family of process discovery algorithms, based on Sander Leemans' PhD thesis *"[Robust Process Mining with Guarantees](https://leemans.ch/publications/theses/phd.pdf)"*.

Given an event log, the library discovers a process tree — a hierarchical model of the process that is **sound by construction**: no deadlocks, no livelocks, clear initial and final states. The library provides formal rediscoverability guarantees.

The library is **case-centric**: it assumes each event belongs to exactly one case, and each case has a single trace. 

This framework offers 8 different ready-to-use Inductive Miner algorithms ([click](#algorithms) for more) and lets users define custom algorithms. Each algorithm is a composition of four stages: **base cases**, **cut detection**, **log splitting**, and **fall-throughs**. Public APIs allow composing custom algorithms from reusable components.

## Table of content

- [Installation](#installation)
- [Quick start](#quick-start)
- [Algorithms](#algorithms)
- [Features](#features)
- [Limitations](#limitations)
- [License](#license)
- [Documentation](#documentation)

## Installation

```toml
[dependencies]
inductive-miner = "0.1"
```
## Quick start
Read an event log from XES file and discover a process tree with the basic Inductive Miner:

```rust
use std::path::Path;
use inductive_miner::{
    algorithms::IM, io::read_xes, EventClassifier, Miner,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let log = read_xes(Path::new("log.xes"), &EventClassifier::default())?;
    let tree = IM::default().mine(&log)?;
    println!("{tree:#?}");
    Ok(())
}
```

Replace `log.xes` with your input file. The default classifier reads activity names from `concept:name` and retains timestamps and lifecycle transitions. Import the `Miner` trait to make the `mine` method available.

Export a discovered tree as JSON:

```rust
let json = tree.to_json_pretty()?;
inductive_miner::io::write_json(std::path::Path::new("tree.json"), &tree)?;
```

JSON contains a `root` with tagged activity, tau, or operator nodes. Operator
children retain their order, including the body-first ordering of loops.

## Algorithms

| Algorithm                             | Purpose                     | Guarantees                          |
| ------------------------------------- | --------------------------- | ----------------------------------- |
| [`IM`](./docs/algorithms/IM.md)       | Basic discovery             | Fitness, rediscoverability (`Cb`)   |
| [`IMf`](./docs/algorithms/IMf.md)     | Filters infrequent behavior | Rediscoverability (`Cb`)            |
| [`IMc`](./docs/algorithms/IMc.md)     | Handles incomplete logs     | Rediscoverability (`Cb`)            |
| [`IMa`](./docs/algorithms/IMa.md)     | Supports `τ`, `↔`, `∨`      | Fitness, rediscoverability (`Ccoo`) |
| [`IMfa`](./docs/algorithms/IMfa.md)   | `IMa` + filtering           | Rediscoverability (`Ccoo`)          |
| [`IMlc`](./docs/algorithms/IMlc.md)   | Non-atomic logs             | Fitness, rediscoverability (`Clc`)  |
| [`IMflc`](./docs/algorithms/IMflc.md) | `IMlc` + filtering          | Rediscoverability (`Clc`)           |
| [`IMclc`](./docs/algorithms/IMclc.md) | `IMlc` + incompleteness     | Rediscoverability (`Clc`)           |

For detailed descriptions, see [`docs/algorithms/`](docs/algorithms/).

## Features

- 11 ready-to-use Inductive Miner variants
- Sound process trees guaranteed by construction
- Formal rediscoverability guarantees for well-defined classes
- Non-atomic event logs supported
- XES I/O
- No unsafe code

## Limitations

- **Case-centric**: each event belongs to exactly one case.
- **Control-flow only**: resources, data, and time constraints are not learned.
- **Rediscoverability is class-limited**: guarantees hold only for `Cb`, `Ccoo`, `Clc`. See the thesis for details.

## License
This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Documentation
<!-- - [API reference]() — rustdoc -->
- [Thesis](https://leemans.ch/publications/theses/phd.pdf) — the source
- [Architecture](docs/architecture.md) — how the framework is organized
- [Algorithms](docs/algorithms/) — per-algorithm details

<!-- ## Performance -->
