use serde::{Deserialize, Serialize};

use super::Tree;

/// A sample plot in the forest inventory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plot {
    /// Unique plot identifier
    pub plot_id: u32,
    /// Plot size in acres
    pub plot_size_acres: f64,
    /// Slope percentage
    pub slope_percent: Option<f64>,
    /// Aspect in degrees (0-360)
    pub aspect_degrees: Option<f64>,
    /// Elevation in feet
    pub elevation_ft: Option<f64>,
    /// Trees measured on this plot
    pub trees: Vec<Tree>,
}

impl Plot {
    /// Get only live trees on this plot.
    pub fn live_trees(&self) -> Vec<&Tree> {
        self.trees.iter().filter(|t| t.is_live()).collect()
    }

    /// Calculate trees per acre for this plot.
    pub fn trees_per_acre(&self) -> f64 {
        let live_count: f64 = self.live_trees().iter().map(|t| t.expansion_factor).sum();
        live_count
    }

    /// Calculate basal area per acre for this plot.
    pub fn basal_area_per_acre(&self) -> f64 {
        self.live_trees()
            .iter()
            .map(|t| t.basal_area_per_acre())
            .sum()
    }

    /// Calculate total cubic foot volume per acre for this plot.
    pub fn volume_cuft_per_acre(&self) -> f64 {
        self.live_trees()
            .iter()
            .filter_map(|t| t.volume_cuft().map(|v| v * t.expansion_factor))
            .sum()
    }

    /// Calculate total board foot volume per acre for this plot.
    pub fn volume_bdft_per_acre(&self) -> f64 {
        self.live_trees()
            .iter()
            .filter_map(|t| t.volume_bdft().map(|v| v * t.expansion_factor))
            .sum()
    }

    /// Calculate quadratic mean diameter (QMD) for live trees.
    pub fn quadratic_mean_diameter(&self) -> f64 {
        let live = self.live_trees();
        if live.is_empty() {
            return 0.0;
        }
        let sum_dbh_sq: f64 = live.iter().map(|t| t.dbh.powi(2) * t.expansion_factor).sum();
        let total_tpa: f64 = live.iter().map(|t| t.expansion_factor).sum();
        if total_tpa == 0.0 {
            return 0.0;
        }
        (sum_dbh_sq / total_tpa).sqrt()
    }
}
