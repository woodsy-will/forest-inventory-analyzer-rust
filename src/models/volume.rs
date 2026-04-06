use serde::{Deserialize, Serialize};

use crate::error::ForestError;

/// Configurable volume equation coefficients.
///
/// Cubic foot volume: `V = cuft_b1 * DBH^2 * H`
/// Board foot volume (Scribner): `V = bdft_b1 * DBH^2 * H - bdft_b2 * DBH`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeEquation {
    /// Coefficient for cubic foot volume: V = cuft_b1 * DBH^2 * H
    pub cuft_b1: f64,
    /// Coefficient for board foot volume: V = bdft_b1 * DBH^2 * H - bdft_b2 * DBH
    pub bdft_b1: f64,
    /// Second coefficient for board foot volume
    pub bdft_b2: f64,
    /// Minimum DBH for board foot merchantability
    pub bdft_min_dbh: f64,
}

impl VolumeEquation {
    /// Validate that all coefficients are finite, non-NaN, and positive (non-zero).
    ///
    /// Returns `Err(ForestError::ValidationError)` if any coefficient is NaN,
    /// infinite, negative, or zero.
    pub fn validate(&self) -> Result<(), ForestError> {
        // Collect all named coefficients for a uniform check
        let fields: &[(&str, f64)] = &[
            ("cuft_b1", self.cuft_b1),
            ("bdft_b1", self.bdft_b1),
            ("bdft_b2", self.bdft_b2),
            ("bdft_min_dbh", self.bdft_min_dbh),
        ];
        for &(name, value) in fields {
            if value.is_nan() {
                return Err(ForestError::ValidationError(format!(
                    "{name} must not be NaN"
                )));
            }
            if value.is_infinite() {
                return Err(ForestError::ValidationError(format!(
                    "{name} must not be infinite"
                )));
            }
            if value <= 0.0 {
                return Err(ForestError::ValidationError(format!(
                    "{name} must be positive, got {value}"
                )));
            }
        }
        Ok(())
    }

    /// Compute gross cubic-foot volume from DBH (inches) and height (feet).
    ///
    /// Formula: `cuft_b1 * dbh^2 * height`
    ///
    /// This is the pure formula; caller is responsible for checking that
    /// `dbh > 0` and `height > 0` before calling (negative/zero inputs
    /// will produce nonsensical results).
    pub fn compute_cuft(&self, dbh: f64, height: f64) -> f64 {
        self.cuft_b1 * dbh.powi(2) * height
    }

    /// Compute gross board-foot volume (Scribner) from DBH (inches) and height (feet).
    ///
    /// Formula: `bdft_b1 * dbh^2 * height - bdft_b2 * dbh`, clamped to >= 0.
    /// Returns 0.0 if `dbh < bdft_min_dbh`.
    ///
    /// This is the pure formula; caller is responsible for ensuring valid inputs.
    pub fn compute_bdft(&self, dbh: f64, height: f64) -> f64 {
        if dbh < self.bdft_min_dbh {
            return 0.0;
        }
        let gross = self.bdft_b1 * dbh.powi(2) * height - self.bdft_b2 * dbh;
        gross.max(0.0)
    }
}

impl Default for VolumeEquation {
    fn default() -> Self {
        Self {
            cuft_b1: 0.002454,
            bdft_b1: 0.01159,
            bdft_b2: 4.0,
            bdft_min_dbh: 6.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_cuft_coefficient() {
        let eq = VolumeEquation::default();
        assert!((eq.cuft_b1 - 0.002454).abs() < 1e-9);
    }

    #[test]
    fn test_default_bdft_coefficients() {
        let eq = VolumeEquation::default();
        assert!((eq.bdft_b1 - 0.01159).abs() < 1e-9);
        assert!((eq.bdft_b2 - 4.0).abs() < 1e-9);
        assert!((eq.bdft_min_dbh - 6.0).abs() < 1e-9);
    }

    // --- validate() tests ---

    #[test]
    fn test_validate_default_ok() {
        assert!(VolumeEquation::default().validate().is_ok());
    }

    #[test]
    fn test_validate_nan_cuft_b1() {
        let eq = VolumeEquation { cuft_b1: f64::NAN, ..VolumeEquation::default() };
        let err = eq.validate().unwrap_err();
        assert!(err.to_string().contains("cuft_b1 must not be NaN"));
    }

    #[test]
    fn test_validate_infinity_bdft_b1() {
        let eq = VolumeEquation { bdft_b1: f64::INFINITY, ..VolumeEquation::default() };
        let err = eq.validate().unwrap_err();
        assert!(err.to_string().contains("bdft_b1 must not be infinite"));
    }

    #[test]
    fn test_validate_neg_infinity() {
        let eq = VolumeEquation { bdft_b2: f64::NEG_INFINITY, ..VolumeEquation::default() };
        let err = eq.validate().unwrap_err();
        assert!(err.to_string().contains("bdft_b2 must not be infinite"));
    }

    #[test]
    fn test_validate_zero_coefficient() {
        let eq = VolumeEquation { cuft_b1: 0.0, ..VolumeEquation::default() };
        let err = eq.validate().unwrap_err();
        assert!(err.to_string().contains("cuft_b1 must be positive"));
    }

    #[test]
    fn test_validate_negative_coefficient() {
        let eq = VolumeEquation { bdft_min_dbh: -1.0, ..VolumeEquation::default() };
        let err = eq.validate().unwrap_err();
        assert!(err.to_string().contains("bdft_min_dbh must be positive"));
    }

    // --- compute_cuft tests ---

    #[test]
    fn test_compute_cuft_basic() {
        let eq = VolumeEquation::default();
        // 0.002454 * 16^2 * 100 = 0.002454 * 256 * 100 = 62.8224
        let vol = eq.compute_cuft(16.0, 100.0);
        assert!((vol - 62.8224).abs() < 0.001);
    }

    #[test]
    fn test_compute_cuft_custom() {
        let eq = VolumeEquation { cuft_b1: 0.003, ..VolumeEquation::default() };
        // 0.003 * 256 * 100 = 76.8
        let vol = eq.compute_cuft(16.0, 100.0);
        assert!((vol - 76.8).abs() < 0.001);
    }

    // --- compute_bdft tests ---

    #[test]
    fn test_compute_bdft_basic() {
        let eq = VolumeEquation::default();
        // 0.01159 * 256 * 100 - 4.0 * 16 = 296.704 - 64 = 232.704
        let vol = eq.compute_bdft(16.0, 100.0);
        assert!((vol - 232.704).abs() < 0.01);
    }

    #[test]
    fn test_compute_bdft_below_min_dbh() {
        let eq = VolumeEquation::default();
        assert_eq!(eq.compute_bdft(5.0, 50.0), 0.0);
    }

    #[test]
    fn test_compute_bdft_clamped_to_zero() {
        let eq = VolumeEquation::default();
        // Very short tree at min dbh: formula may go negative, should clamp
        let vol = eq.compute_bdft(6.0, 1.0);
        assert!(vol >= 0.0);
    }

    #[test]
    fn test_compute_bdft_at_min_dbh_boundary() {
        let eq = VolumeEquation::default();
        // Exactly at bdft_min_dbh (6.0) should compute, not return 0
        let vol = eq.compute_bdft(6.0, 100.0);
        // 0.01159 * 36 * 100 - 4.0 * 6 = 41.724 - 24 = 17.724
        assert!((vol - 17.724).abs() < 0.01);
    }

    #[test]
    fn test_volume_equation_json_roundtrip() {
        let eq = VolumeEquation {
            cuft_b1: 0.003,
            bdft_b1: 0.012,
            bdft_b2: 3.5,
            bdft_min_dbh: 5.0,
        };
        let json = serde_json::to_string(&eq).unwrap();
        let deserialized: VolumeEquation = serde_json::from_str(&json).unwrap();
        assert!((deserialized.cuft_b1 - 0.003).abs() < 1e-9);
        assert!((deserialized.bdft_b1 - 0.012).abs() < 1e-9);
    }
}
