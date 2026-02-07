use serde::{Deserialize, Serialize};

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
        let height = self.height?;
        if self.dbh <= 0.0 || height <= 0.0 {
            return Some(0.0);
        }
        // Simplified combined variable equation: V = b1 * DBH^2 * H
        // Using a general coefficient; species-specific coefficients would improve accuracy
        let b1 = 0.002454;
        let gross_volume = b1 * self.dbh.powi(2) * height;
        let defect_factor = 1.0 - self.defect.unwrap_or(0.0);
        Some(gross_volume * defect_factor)
    }

    /// Estimate board foot volume (Scribner) using a simplified equation.
    pub fn volume_bdft(&self) -> Option<f64> {
        let height = self.height?;
        if self.dbh < 6.0 || height <= 0.0 {
            return Some(0.0);
        }
        // Simplified Scribner board foot volume
        let b1 = 0.01159;
        let gross_volume = b1 * self.dbh.powi(2) * height - 4.0 * self.dbh;
        let defect_factor = 1.0 - self.defect.unwrap_or(0.0);
        Some(gross_volume.max(0.0) * defect_factor)
    }

    /// Check if the tree is alive.
    pub fn is_live(&self) -> bool {
        self.status == TreeStatus::Live
    }
}
