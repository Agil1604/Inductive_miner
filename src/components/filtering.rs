/// Shared frequency-filtering configuration for process discovery.
///
/// Stores a finite threshold in `[0, 1]`, defaulting to `0.2`. The same value
/// controls DFG filtering, the single-activity estimate, and empty-trace handling.
/// Build a miner with [`Self::imf`], [`Self::imfa`], or [`Self::imflc`].
///
/// ```
/// use robust_process_mining_with_guarantees::{FilteringConfig, EventLog, Miner};
/// let config = FilteringConfig::new(0.15)?;
/// let imf = config.imf();
/// let imfa = config.imfa();
/// imf.mine(&EventLog::default())?;
/// imfa.mine(&EventLog::default())?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FilteringConfig {
    /// Validated relative deviation threshold.
    deviation_threshold: f64,
}

/// A nonfinite threshold or a value outside the inclusive interval `[0, 1]`.
#[derive(Debug, Clone, Copy, PartialEq, thiserror::Error)]
#[error("Deviation threshold must be finite and between 0 and 1 inclusive; got {0}")]
pub struct InvalidDeviationThreshold(
    /// Rejected threshold value.
    pub f64,
);

impl FilteringConfig {
    /// Validates and stores the deviation threshold.
    ///
    /// # Errors
    /// Returns [`InvalidDeviationThreshold`] for NaN, infinity, or values outside
    /// the inclusive interval `[0, 1]`.
    pub fn new(deviation_threshold: f64) -> Result<Self, InvalidDeviationThreshold> {
        if !deviation_threshold.is_finite() || !(0.0..=1.0).contains(&deviation_threshold) {
            return Err(InvalidDeviationThreshold(deviation_threshold));
        }
        Ok(Self {
            deviation_threshold,
        })
    }

    /// Getter for the deviation threshold.
    pub fn deviation_threshold(self) -> f64 {
        self.deviation_threshold
    }
}

impl Default for FilteringConfig {
    /// Returns a default configuration with a deviation threshold of `0.2`.
    fn default() -> Self {
        Self {
            deviation_threshold: 0.2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_boundaries_and_fractional_thresholds() {
        for value in [0.0, 0.15, 0.2, 1.0] {
            assert_eq!(
                FilteringConfig::new(value).unwrap().deviation_threshold(),
                value
            );
        }
    }

    #[test]
    fn rejects_out_of_range_and_non_finite_thresholds() {
        for value in [-0.01, 1.01, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(FilteringConfig::new(value).is_err());
        }
    }

    #[test]
    fn default_is_valid_and_uses_point_two() {
        assert_eq!(
            FilteringConfig::default(),
            FilteringConfig::new(0.2).unwrap()
        );
    }
}
