use super::imflc::ImflcLogSplitter;
use super::imlc::ImlcBaseCase;
use crate::InductiveMiner;
use crate::components::cut_detections::ImclcCutFinder;
use crate::components::fall_throughs::{ConcurrentFlowerModel, EmptyTraces, FallThroughFinder};

/// Lifecycle-aware Inductive Miner for incomplete behaviour.
/// Construct with `IMclc::default()` and import [`crate::Miner`] to call `mine`.
/// Uses IMlc cuts first, then SMT; filtered splitting can discard behaviour.
pub type IMclc = InductiveMiner<ImlcBaseCase, ImclcCutFinder, ImflcLogSplitter, ImclcFallThrough>;

pub type ImclcFallThrough = FallThroughFinder<EmptyTraces, ConcurrentFlowerModel>;
#[cfg(test)]
mod tests;
