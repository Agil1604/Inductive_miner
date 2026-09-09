# Robust process mining with guarantees

# Table of content
1. [About](#about)

## About

`inductive-miner` is a Rust implementation of the Inductive Miner family of
process discovery algorithms, based on the framework from Sander Leemans'
PhD thesis *"[Robust Process Mining with Guarantees](https://leemans.ch/publications/theses/phd.pdf)"*.

Given an event log, the library discovers a process tree — a hierarchical
model of the process that is **sound by construction** (no deadlocks, no
livelocks, clear initial and final states). Unlike many existing tools,
it guarantees soundness, provides formal rediscoverability guarantees.

The library is **case-centric**: it assumes each event belongs to exactly
one case, and each case has a single trace. 

The framework separates discovery into four stages: base cases, cut detection, log splitting, and fall-throughs. Reusable components implement these stages and can be combined to construct different mining algorithms.


## Quick start
