use serde::{Deserialize, Serialize};

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
