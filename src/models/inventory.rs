use serde::{Deserialize, Serialize};

use super::{Plot, Species};

/// A complete forest inventory dataset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForestInventory {
    /// Name or identifier for this inventory
    pub name: String,
    /// Total area in acres
    pub total_acres: Option<f64>,
    /// All plots in the inventory
    pub plots: Vec<Plot>,
}

impl ForestInventory {
    /// Create a new empty inventory.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            total_acres: None,
            plots: Vec::new(),
        }
    }

    /// Get all unique species across the inventory.
    pub fn species_list(&self) -> Vec<Species> {
        let mut species: Vec<Species> = self
            .plots
            .iter()
            .flat_map(|p| p.trees.iter().map(|t| t.species.clone()))
            .collect();
        species.sort_by(|a, b| a.code.cmp(&b.code));
        species.dedup_by(|a, b| a.code == b.code);
        species
    }

    /// Total number of plots.
    pub fn num_plots(&self) -> usize {
        self.plots.len()
    }

    /// Total number of measured trees.
    pub fn num_trees(&self) -> usize {
        self.plots.iter().map(|p| p.trees.len()).sum()
    }

    /// Mean trees per acre across all plots.
    pub fn mean_tpa(&self) -> f64 {
        if self.plots.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.plots.iter().map(|p| p.trees_per_acre()).sum();
        sum / self.plots.len() as f64
    }

    /// Mean basal area per acre across all plots.
    pub fn mean_basal_area(&self) -> f64 {
        if self.plots.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.plots.iter().map(|p| p.basal_area_per_acre()).sum();
        sum / self.plots.len() as f64
    }

    /// Mean cubic foot volume per acre across all plots.
    pub fn mean_volume_cuft(&self) -> f64 {
        if self.plots.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.plots.iter().map(|p| p.volume_cuft_per_acre()).sum();
        sum / self.plots.len() as f64
    }

    /// Mean board foot volume per acre across all plots.
    pub fn mean_volume_bdft(&self) -> f64 {
        if self.plots.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.plots.iter().map(|p| p.volume_bdft_per_acre()).sum();
        sum / self.plots.len() as f64
    }
}
