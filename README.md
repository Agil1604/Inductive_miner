# Inductive Miner

## Table of content
- [Inductive Miner](#inductive-miner)
  - [Table of content](#table-of-content)
  - [About](#about)
  - [Quick start](#quick-start)
  - [Algorithms](#algorithms)

## About

`inductive-miner` is a Rust framework implementing the Inductive Miner family of process discovery algorithms, based on the Sander Leemans' PhD thesis *"[Robust Process Mining with Guarantees](https://leemans.ch/publications/theses/phd.pdf)"*.

Given an event log, the library discovers a process tree — a hierarchical
model of the process that is **sound by construction** (no deadlocks, no
livelocks, clear initial and final states). Unlike many existing tools,
it guarantees soundness, provides formal rediscoverability guarantees.

The library is **case-centric**: it assumes each event belongs to exactly
one case, and each case has a single trace. 

This framework offers 11 different ready-to-use Inductive Miner algorithms ([click](#algorithms) for more) and lets user easily define custom algorithms.

It is achieved by separating discovery into four stages: **base cases**, **cut detection**, **log splitting**, and **fall-throughs**. Reusable components implement these stages and can be combined by many utils awailable in the public API.

## Quick start

Add the library as a local dependency, replacing the path with your checkout:

```toml
[dependencies]
inductive_miner = { path = "./path/to/project" }
```

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

Replace `log.xes` with your input file. The default classifier reads activity
names from `concept:name` and retains timestamps and lifecycle transitions.
Import the `Miner` trait to make the `mine` method available.

## Algorithms