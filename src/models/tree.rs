use serde::{Deserialize, Serialize};

use super::volume::VolumeEquation;

/// Status of a tree in the inventory.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TreeStatus {
    Live,
    Dead,
    Cut,
    Missing,
}

impl std::fmt::Display for TreeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TreeStatus::Live => write!(f, "Live"),
            TreeStatus::Dead => write!(f, "Dead"),
            TreeStatus::Cut => write!(f, "Cut"),
            TreeStatus::Missing => write!(f, "Missing"),
        }
    }
}

impl std::str::FromStr for TreeStatus {
    type Err = crate::error::ForestError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "live" | "l" => Ok(TreeStatus::Live),
            "dead" | "d" => Ok(TreeStatus::Dead),
            "cut" | "c" => Ok(TreeStatus::Cut),
            "missing" | "m" => Ok(TreeStatus::Missing),
            _ => Err(crate::error::ForestError::ParseError(format!(
                "Unknown tree status: '{s}'"
            ))),
        }
    }
}

/// Species information for a tree.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Species {
    /// Common name (e.g., "Douglas Fir")
    pub common_name: String,
    /// Species code (e.g., "DF", "PSME")
    pub code: String,
}

impl std::fmt::Display for Species {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.common_name, self.code)
    }
}

/// A single tree measurement record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tree {
    /// Unique tree identifier within the plot
    pub tree_id: u32,
    /// Plot this tree belongs to
    pub plot_id: u32,
    /// Species information
    pub species: Species,
    /// Diameter at breast height in inches
    pub dbh: f64,
    /// Total height in feet
    pub height: Option<f64>,
    /// Crown ratio (0.0 - 1.0)
    pub crown_ratio: Option<f64>,
    /// Tree status
    pub status: TreeStatus,
    /// Number of trees this sample tree represents (expansion factor)
    pub expansion_factor: f64,
    /// Age at breast height (if cored)
    pub age: Option<u32>,
    /// Defect percentage (0.0 - 1.0)
    pub defect: Option<f64>,
}

impl Tree {
    /// Calculate basal area in square feet for this tree.
    pub fn basal_area_sqft(&self) -> f64 {
        std::f64::consts::PI * (self.dbh / 2.0).powi(2) / 144.0
    }

    /// Calculate basal area per acre using the expansion factor.
    pub fn basal_area_per_acre(&self) -> f64 {
        self.basal_area_sqft() * self.expansion_factor
    }

    /// Estimate cubic foot volume using the combined variable equation.
    /// Uses a simplified form of the National Volume Estimator approach.
    pub fn volume_cuft(&self) -> Option<f64> {
        self.volume_cuft_with(&VolumeEquation::default())
    }

    /// Estimate cubic foot volume using custom equation coefficients.
    pub fn volume_cuft_with(&self, eq: &VolumeEquation) -> Option<f64> {
        let height = self.height?;
        if self.dbh <= 0.0 || height <= 0.0 {
            return Some(0.0);
        }
        let gross_volume = eq.cuft_b1 * self.dbh.powi(2) * height;
        let defect_factor = 1.0 - self.defect.unwrap_or(0.0);
        Some(gross_volume * defect_factor)
    }

    /// Estimate board foot volume (Scribner) using a simplified equation.
    pub fn volume_bdft(&self) -> Option<f64> {
        self.volume_bdft_with(&VolumeEquation::default())
    }

    /// Estimate board foot volume using custom equation coefficients.
    pub fn volume_bdft_with(&self, eq: &VolumeEquation) -> Option<f64> {
        let height = self.height?;
        if self.dbh < eq.bdft_min_dbh || height <= 0.0 {
            return Some(0.0);
        }
        let gross_volume = eq.bdft_b1 * self.dbh.powi(2) * height - eq.bdft_b2 * self.dbh;
        let defect_factor = 1.0 - self.defect.unwrap_or(0.0);
        Some(gross_volume.max(0.0) * defect_factor)
    }

    /// Check if the tree is alive.
    pub fn is_live(&self) -> bool {
        self.status == TreeStatus::Live
    }

    /// Validate tree measurements. Returns `ForestError::ValidationError` on failure.
    pub fn validate(&self) -> Result<(), crate::error::ForestError> {
        if self.dbh <= 0.0 {
            return Err(crate::error::ForestError::ValidationError(format!(
                "Plot {}, Tree {}: DBH must be positive, got {}",
                self.plot_id, self.tree_id, self.dbh
            )));
        }
        if let Some(h) = self.height {
            if h <= 0.0 {
                return Err(crate::error::ForestError::ValidationError(format!(
                    "Plot {}, Tree {}: height must be positive, got {}",
                    self.plot_id, self.tree_id, h
                )));
            }
        }
        if let Some(cr) = self.crown_ratio {
            if !(0.0..=1.0).contains(&cr) {
                return Err(crate::error::ForestError::ValidationError(format!(
                    "Plot {}, Tree {}: crown_ratio must be in 0.0..=1.0, got {}",
                    self.plot_id, self.tree_id, cr
                )));
            }
        }
        if self.expansion_factor <= 0.0 {
            return Err(crate::error::ForestError::ValidationError(format!(
                "Plot {}, Tree {}: expansion_factor must be positive, got {}",
                self.plot_id, self.tree_id, self.expansion_factor
            )));
        }
        if let Some(d) = self.defect {
            if !(0.0..=1.0).contains(&d) {
                return Err(crate::error::ForestError::ValidationError(format!(
                    "Plot {}, Tree {}: defect must be in 0.0..=1.0, got {}",
                    self.plot_id, self.tree_id, d
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tree(dbh: f64, height: Option<f64>, status: TreeStatus, ef: f64) -> Tree {
        Tree {
            tree_id: 1,
            plot_id: 1,
            species: Species {
                common_name: "Douglas Fir".to_string(),
                code: "DF".to_string(),
            },
            dbh,
            height,
            crown_ratio: Some(0.5),
            status,
            expansion_factor: ef,
            age: Some(60),
            defect: None,
        }
    }

    // --- TreeStatus tests ---

    #[test]
    fn test_tree_status_display() {
        assert_eq!(TreeStatus::Live.to_string(), "Live");
        assert_eq!(TreeStatus::Dead.to_string(), "Dead");
        assert_eq!(TreeStatus::Cut.to_string(), "Cut");
        assert_eq!(TreeStatus::Missing.to_string(), "Missing");
    }

    #[test]
    fn test_tree_status_parse_full_words() {
        assert_eq!("live".parse::<TreeStatus>().unwrap(), TreeStatus::Live);
        assert_eq!("dead".parse::<TreeStatus>().unwrap(), TreeStatus::Dead);
        assert_eq!("cut".parse::<TreeStatus>().unwrap(), TreeStatus::Cut);
        assert_eq!("missing".parse::<TreeStatus>().unwrap(), TreeStatus::Missing);
    }

    #[test]
    fn test_tree_status_parse_abbreviations() {
        assert_eq!("l".parse::<TreeStatus>().unwrap(), TreeStatus::Live);
        assert_eq!("d".parse::<TreeStatus>().unwrap(), TreeStatus::Dead);
        assert_eq!("c".parse::<TreeStatus>().unwrap(), TreeStatus::Cut);
        assert_eq!("m".parse::<TreeStatus>().unwrap(), TreeStatus::Missing);
    }

    #[test]
    fn test_tree_status_parse_case_insensitive() {
        assert_eq!("LIVE".parse::<TreeStatus>().unwrap(), TreeStatus::Live);
        assert_eq!("Live".parse::<TreeStatus>().unwrap(), TreeStatus::Live);
        assert_eq!("L".parse::<TreeStatus>().unwrap(), TreeStatus::Live);
        assert_eq!("DEAD".parse::<TreeStatus>().unwrap(), TreeStatus::Dead);
    }

    #[test]
    fn test_tree_status_parse_invalid() {
        assert!("unknown".parse::<TreeStatus>().is_err());
        assert!("alive".parse::<TreeStatus>().is_err());
        assert!("".parse::<TreeStatus>().is_err());
        assert!("x".parse::<TreeStatus>().is_err());
    }

    // --- Species tests ---

    #[test]
    fn test_species_display() {
        let sp = Species {
            common_name: "Douglas Fir".to_string(),
            code: "DF".to_string(),
        };
        assert_eq!(sp.to_string(), "Douglas Fir (DF)");
    }

    #[test]
    fn test_species_equality() {
        let sp1 = Species {
            common_name: "Douglas Fir".to_string(),
            code: "DF".to_string(),
        };
        let sp2 = Species {
            common_name: "Douglas Fir".to_string(),
            code: "DF".to_string(),
        };
        assert_eq!(sp1, sp2);
    }

    #[test]
    fn test_species_inequality() {
        let sp1 = Species {
            common_name: "Douglas Fir".to_string(),
            code: "DF".to_string(),
        };
        let sp2 = Species {
            common_name: "Western Red Cedar".to_string(),
            code: "WRC".to_string(),
        };
        assert_ne!(sp1, sp2);
    }

    #[test]
    fn test_species_hash_consistency() {
        use std::collections::HashSet;
        let sp1 = Species {
            common_name: "Douglas Fir".to_string(),
            code: "DF".to_string(),
        };
        let sp2 = sp1.clone();
        let mut set = HashSet::new();
        set.insert(sp1);
        set.insert(sp2);
        assert_eq!(set.len(), 1);
    }

    // --- Basal area tests ---

    #[test]
    fn test_basal_area_12_inch_tree() {
        let tree = make_tree(12.0, Some(80.0), TreeStatus::Live, 5.0);
        let ba = tree.basal_area_sqft();
        // BA = pi * (12/2)^2 / 144 = pi * 36 / 144 = 0.7854
        assert!((ba - 0.7854).abs() < 0.001);
    }

    #[test]
    fn test_basal_area_zero_dbh() {
        let tree = make_tree(0.0, Some(80.0), TreeStatus::Live, 5.0);
        assert_eq!(tree.basal_area_sqft(), 0.0);
    }

    #[test]
    fn test_basal_area_per_acre() {
        let tree = make_tree(12.0, Some(80.0), TreeStatus::Live, 5.0);
        let ba = tree.basal_area_sqft();
        let ba_per_acre = tree.basal_area_per_acre();
        assert!((ba_per_acre - ba * 5.0).abs() < 0.0001);
    }

    #[test]
    fn test_basal_area_large_tree() {
        let tree = make_tree(36.0, Some(150.0), TreeStatus::Live, 3.0);
        let ba = tree.basal_area_sqft();
        // BA = pi * (36/2)^2 / 144 = pi * 324 / 144 = 7.069
        assert!((ba - 7.069).abs() < 0.01);
    }

    #[test]
    fn test_basal_area_small_tree() {
        let tree = make_tree(1.0, Some(15.0), TreeStatus::Live, 20.0);
        let ba = tree.basal_area_sqft();
        // BA = pi * (0.5)^2 / 144 = pi * 0.25 / 144 ≈ 0.00545
        assert!((ba - 0.00545).abs() < 0.001);
    }

    // --- Volume tests ---

    #[test]
    fn test_volume_cuft_normal_tree() {
        let tree = make_tree(16.0, Some(100.0), TreeStatus::Live, 5.0);
        let vol = tree.volume_cuft().unwrap();
        // V = 0.002454 * 16^2 * 100 = 0.002454 * 256 * 100 = 62.82
        assert!((vol - 62.82).abs() < 0.1);
    }

    #[test]
    fn test_volume_cuft_no_height() {
        let tree = make_tree(16.0, None, TreeStatus::Live, 5.0);
        assert!(tree.volume_cuft().is_none());
    }

    #[test]
    fn test_volume_cuft_zero_dbh() {
        let tree = make_tree(0.0, Some(100.0), TreeStatus::Live, 5.0);
        assert_eq!(tree.volume_cuft().unwrap(), 0.0);
    }

    #[test]
    fn test_volume_cuft_zero_height() {
        let tree = make_tree(16.0, Some(0.0), TreeStatus::Live, 5.0);
        assert_eq!(tree.volume_cuft().unwrap(), 0.0);
    }

    #[test]
    fn test_volume_cuft_with_defect() {
        let mut tree = make_tree(16.0, Some(100.0), TreeStatus::Live, 5.0);
        tree.defect = Some(0.10); // 10% defect
        let vol = tree.volume_cuft().unwrap();
        let gross = 0.002454 * 256.0 * 100.0;
        let expected = gross * 0.90;
        assert!((vol - expected).abs() < 0.1);
    }

    #[test]
    fn test_volume_bdft_normal_tree() {
        let tree = make_tree(16.0, Some(100.0), TreeStatus::Live, 5.0);
        let vol = tree.volume_bdft().unwrap();
        assert!(vol > 0.0);
    }

    #[test]
    fn test_volume_bdft_small_tree_below_merchantable() {
        // Trees < 6" DBH should return 0 board feet
        let tree = make_tree(5.0, Some(50.0), TreeStatus::Live, 10.0);
        assert_eq!(tree.volume_bdft().unwrap(), 0.0);
    }

    #[test]
    fn test_volume_bdft_no_height() {
        let tree = make_tree(16.0, None, TreeStatus::Live, 5.0);
        assert!(tree.volume_bdft().is_none());
    }

    #[test]
    fn test_volume_bdft_with_defect() {
        let mut tree = make_tree(16.0, Some(100.0), TreeStatus::Live, 5.0);
        let vol_no_defect = tree.volume_bdft().unwrap();
        tree.defect = Some(0.20);
        let vol_with_defect = tree.volume_bdft().unwrap();
        assert!((vol_with_defect - vol_no_defect * 0.80).abs() < 0.1);
    }

    #[test]
    fn test_volume_bdft_negative_clamped_to_zero() {
        // Very small merchantable tree where equation might go negative
        let tree = make_tree(6.0, Some(1.0), TreeStatus::Live, 5.0);
        let vol = tree.volume_bdft().unwrap();
        assert!(vol >= 0.0);
    }

    // --- is_live tests ---

    #[test]
    fn test_is_live() {
        assert!(make_tree(12.0, Some(80.0), TreeStatus::Live, 5.0).is_live());
        assert!(!make_tree(12.0, Some(80.0), TreeStatus::Dead, 5.0).is_live());
        assert!(!make_tree(12.0, Some(80.0), TreeStatus::Cut, 5.0).is_live());
        assert!(!make_tree(12.0, Some(80.0), TreeStatus::Missing, 5.0).is_live());
    }

    // --- Serialization roundtrip ---

    #[test]
    fn test_tree_json_roundtrip() {
        let tree = make_tree(16.0, Some(100.0), TreeStatus::Live, 5.0);
        let json = serde_json::to_string(&tree).unwrap();
        let deserialized: Tree = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.tree_id, tree.tree_id);
        assert_eq!(deserialized.dbh, tree.dbh);
        assert_eq!(deserialized.status, tree.status);
    }

    #[test]
    fn test_tree_status_json_roundtrip() {
        for status in &[TreeStatus::Live, TreeStatus::Dead, TreeStatus::Cut, TreeStatus::Missing] {
            let json = serde_json::to_string(status).unwrap();
            let deserialized: TreeStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(&deserialized, status);
        }
    }

    #[test]
    fn test_species_json_roundtrip() {
        let sp = Species {
            common_name: "Western Hemlock".to_string(),
            code: "WH".to_string(),
        };
        let json = serde_json::to_string(&sp).unwrap();
        let deserialized: Species = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, sp);
    }

    // --- Validation tests ---

    #[test]
    fn test_validate_valid_tree() {
        let tree = make_tree(12.0, Some(80.0), TreeStatus::Live, 5.0);
        assert!(tree.validate().is_ok());
    }

    #[test]
    fn test_validate_valid_tree_no_optionals() {
        let mut tree = make_tree(12.0, None, TreeStatus::Live, 5.0);
        tree.crown_ratio = None;
        tree.defect = None;
        assert!(tree.validate().is_ok());
    }

    #[test]
    fn test_validate_zero_dbh() {
        let tree = make_tree(0.0, Some(80.0), TreeStatus::Live, 5.0);
        let err = tree.validate().unwrap_err();
        assert!(err.to_string().contains("DBH must be positive"));
    }

    #[test]
    fn test_validate_negative_dbh() {
        let tree = make_tree(-1.0, Some(80.0), TreeStatus::Live, 5.0);
        let err = tree.validate().unwrap_err();
        assert!(err.to_string().contains("DBH must be positive"));
    }

    #[test]
    fn test_validate_zero_height() {
        let tree = make_tree(12.0, Some(0.0), TreeStatus::Live, 5.0);
        let err = tree.validate().unwrap_err();
        assert!(err.to_string().contains("height must be positive"));
    }

    #[test]
    fn test_validate_negative_height() {
        let tree = make_tree(12.0, Some(-5.0), TreeStatus::Live, 5.0);
        let err = tree.validate().unwrap_err();
        assert!(err.to_string().contains("height must be positive"));
    }

    #[test]
    fn test_validate_crown_ratio_above_one() {
        let mut tree = make_tree(12.0, Some(80.0), TreeStatus::Live, 5.0);
        tree.crown_ratio = Some(1.5);
        let err = tree.validate().unwrap_err();
        assert!(err.to_string().contains("crown_ratio must be in 0.0..=1.0"));
    }

    #[test]
    fn test_validate_crown_ratio_negative() {
        let mut tree = make_tree(12.0, Some(80.0), TreeStatus::Live, 5.0);
        tree.crown_ratio = Some(-0.1);
        let err = tree.validate().unwrap_err();
        assert!(err.to_string().contains("crown_ratio must be in 0.0..=1.0"));
    }

    #[test]
    fn test_validate_crown_ratio_boundary_values() {
        let mut tree = make_tree(12.0, Some(80.0), TreeStatus::Live, 5.0);
        tree.crown_ratio = Some(0.0);
        assert!(tree.validate().is_ok());
        tree.crown_ratio = Some(1.0);
        assert!(tree.validate().is_ok());
    }

    #[test]
    fn test_validate_zero_expansion_factor() {
        let tree = make_tree(12.0, Some(80.0), TreeStatus::Live, 0.0);
        let err = tree.validate().unwrap_err();
        assert!(err.to_string().contains("expansion_factor must be positive"));
    }

    #[test]
    fn test_validate_negative_expansion_factor() {
        let tree = make_tree(12.0, Some(80.0), TreeStatus::Live, -1.0);
        let err = tree.validate().unwrap_err();
        assert!(err.to_string().contains("expansion_factor must be positive"));
    }

    #[test]
    fn test_validate_defect_above_one() {
        let mut tree = make_tree(12.0, Some(80.0), TreeStatus::Live, 5.0);
        tree.defect = Some(1.1);
        let err = tree.validate().unwrap_err();
        assert!(err.to_string().contains("defect must be in 0.0..=1.0"));
    }

    #[test]
    fn test_validate_defect_negative() {
        let mut tree = make_tree(12.0, Some(80.0), TreeStatus::Live, 5.0);
        tree.defect = Some(-0.05);
        let err = tree.validate().unwrap_err();
        assert!(err.to_string().contains("defect must be in 0.0..=1.0"));
    }

    #[test]
    fn test_validate_defect_boundary_values() {
        let mut tree = make_tree(12.0, Some(80.0), TreeStatus::Live, 5.0);
        tree.defect = Some(0.0);
        assert!(tree.validate().is_ok());
        tree.defect = Some(1.0);
        assert!(tree.validate().is_ok());
    }

    // --- volume_cuft_with / volume_bdft_with tests ---

    #[test]
    fn test_volume_cuft_with_default_matches_original() {
        let tree = make_tree(16.0, Some(100.0), TreeStatus::Live, 5.0);
        let default_eq = super::VolumeEquation::default();
        assert_eq!(tree.volume_cuft(), tree.volume_cuft_with(&default_eq));
    }

    #[test]
    fn test_volume_bdft_with_default_matches_original() {
        let tree = make_tree(16.0, Some(100.0), TreeStatus::Live, 5.0);
        let default_eq = super::VolumeEquation::default();
        assert_eq!(tree.volume_bdft(), tree.volume_bdft_with(&default_eq));
    }

    #[test]
    fn test_volume_cuft_with_custom_coefficients() {
        let tree = make_tree(16.0, Some(100.0), TreeStatus::Live, 5.0);
        let eq = super::VolumeEquation {
            cuft_b1: 0.003,
            ..super::VolumeEquation::default()
        };
        let vol = tree.volume_cuft_with(&eq).unwrap();
        // V = 0.003 * 16^2 * 100 = 76.8
        assert!((vol - 76.8).abs() < 0.1);
    }

    #[test]
    fn test_volume_bdft_with_custom_coefficients() {
        let tree = make_tree(16.0, Some(100.0), TreeStatus::Live, 5.0);
        let eq = super::VolumeEquation {
            bdft_b1: 0.015,
            bdft_b2: 5.0,
            bdft_min_dbh: 6.0,
            ..super::VolumeEquation::default()
        };
        let vol = tree.volume_bdft_with(&eq).unwrap();
        // V = 0.015 * 256 * 100 - 5.0 * 16 = 384 - 80 = 304
        assert!((vol - 304.0).abs() < 0.1);
    }

    #[test]
    fn test_volume_bdft_with_custom_min_dbh() {
        let tree = make_tree(8.0, Some(60.0), TreeStatus::Live, 5.0);
        // Default min_dbh=6.0 should give volume
        assert!(tree.volume_bdft().unwrap() > 0.0);
        // Custom min_dbh=10.0 should give 0 for an 8" tree
        let eq = super::VolumeEquation {
            bdft_min_dbh: 10.0,
            ..super::VolumeEquation::default()
        };
        assert_eq!(tree.volume_bdft_with(&eq).unwrap(), 0.0);
    }
}
