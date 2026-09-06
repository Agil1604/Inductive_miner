
///
/// Configuration for the IMf algorithm.
/// 
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImfConfig {
    deviation_threshold: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, thiserror::Error)]
#[error("IMf deviation threshold must be finite and between 0 and 1 inclusive; got {0}")]
pub struct InvalidDeviationThreshold(pub f64);

impl ImfConfig {
    /// Validates and stores the deviation threshold.
    ///
    /// # Errors
    /// Returns [`InvalidDeviationThreshold`] for NaN, infinity, or values outside
    /// the inclusive interval `[0, 1]`.
    ///
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

impl Default for ImfConfig {
    ///
    /// Returns a default configuration with a deviation threshold of `0.2`.
    ///
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
            assert_eq!(ImfConfig::new(value).unwrap().deviation_threshold(), value);
        }
    }

    #[test]
    fn rejects_out_of_range_and_non_finite_thresholds() {
        for value in [-0.01, 1.01, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(ImfConfig::new(value).is_err());
        }
    }

    #[test]
    fn default_is_valid_and_uses_point_two() {
        assert_eq!(ImfConfig::default(), ImfConfig::new(0.2).unwrap());
    }
}
