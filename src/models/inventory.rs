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
    ///
    /// # Examples
    ///
    /// ```
    /// use forest_inventory_analyzer::{ForestInventory, Plot, Tree, Species, TreeStatus};
    ///
    /// let mut inv = ForestInventory::new("Example");
    /// inv.plots.push(Plot {
    ///     plot_id: 1, plot_size_acres: 0.2,
    ///     slope_percent: None, aspect_degrees: None, elevation_ft: None,
    ///     trees: vec![Tree {
    ///         tree_id: 1, plot_id: 1,
    ///         species: Species { common_name: "Douglas Fir".into(), code: "DF".into() },
    ///         dbh: 14.0, height: Some(90.0), crown_ratio: None,
    ///         status: TreeStatus::Live, expansion_factor: 5.0, age: None, defect: None,
    ///     }],
    /// });
    /// assert!((inv.mean_tpa() - 5.0).abs() < 0.001);
    /// ```
    pub fn mean_tpa(&self) -> f64 {
        if self.plots.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.plots.iter().map(|p| p.trees_per_acre()).sum();
        sum / self.plots.len() as f64
    }

    /// Mean basal area per acre across all plots (sq ft/acre).
    ///
    /// # Examples
    ///
    /// ```
    /// use forest_inventory_analyzer::{ForestInventory, Plot, Tree, Species, TreeStatus};
    ///
    /// let mut inv = ForestInventory::new("Example");
    /// inv.plots.push(Plot {
    ///     plot_id: 1, plot_size_acres: 0.2,
    ///     slope_percent: None, aspect_degrees: None, elevation_ft: None,
    ///     trees: vec![Tree {
    ///         tree_id: 1, plot_id: 1,
    ///         species: Species { common_name: "Douglas Fir".into(), code: "DF".into() },
    ///         dbh: 14.0, height: Some(90.0), crown_ratio: None,
    ///         status: TreeStatus::Live, expansion_factor: 5.0, age: None, defect: None,
    ///     }],
    /// });
    /// assert!(inv.mean_basal_area() > 0.0);
    /// ```
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Tree, TreeStatus};

    fn make_species(code: &str, name: &str) -> Species {
        Species {
            common_name: name.to_string(),
            code: code.to_string(),
        }
    }

    fn make_tree(plot_id: u32, species: Species, dbh: f64, status: TreeStatus) -> Tree {
        Tree {
            tree_id: 1,
            plot_id,
            species,
            dbh,
            height: Some(80.0),
            crown_ratio: Some(0.5),
            status,
            expansion_factor: 5.0,
            age: None,
            defect: None,
        }
    }

    fn make_plot_with_trees(plot_id: u32, trees: Vec<Tree>) -> Plot {
        Plot {
            plot_id,
            plot_size_acres: 0.2,
            slope_percent: None,
            aspect_degrees: None,
            elevation_ft: None,
            trees,
        }
    }

    fn sample_inventory() -> ForestInventory {
        let df = make_species("DF", "Douglas Fir");
        let wrc = make_species("WRC", "Western Red Cedar");

        let mut inv = ForestInventory::new("Test");
        inv.plots.push(make_plot_with_trees(
            1,
            vec![
                make_tree(1, df.clone(), 16.0, TreeStatus::Live),
                make_tree(1, wrc.clone(), 12.0, TreeStatus::Live),
            ],
        ));
        inv.plots.push(make_plot_with_trees(
            2,
            vec![
                make_tree(2, df.clone(), 18.0, TreeStatus::Live),
                make_tree(2, df.clone(), 8.0, TreeStatus::Dead),
            ],
        ));
        inv
    }

    #[test]
    fn test_new_inventory() {
        let inv = ForestInventory::new("My Inventory");
        assert_eq!(inv.name, "My Inventory");
        assert!(inv.total_acres.is_none());
        assert!(inv.plots.is_empty());
    }

    #[test]
    fn test_new_inventory_string_conversion() {
        let inv = ForestInventory::new(String::from("Owned String"));
        assert_eq!(inv.name, "Owned String");
    }

    #[test]
    fn test_num_plots() {
        let inv = sample_inventory();
        assert_eq!(inv.num_plots(), 2);
    }

    #[test]
    fn test_num_plots_empty() {
        let inv = ForestInventory::new("Empty");
        assert_eq!(inv.num_plots(), 0);
    }

    #[test]
    fn test_num_trees() {
        let inv = sample_inventory();
        assert_eq!(inv.num_trees(), 4); // 2 + 2
    }

    #[test]
    fn test_num_trees_empty() {
        let inv = ForestInventory::new("Empty");
        assert_eq!(inv.num_trees(), 0);
    }

    #[test]
    fn test_species_list() {
        let inv = sample_inventory();
        let species = inv.species_list();
        assert_eq!(species.len(), 2);
        // Should be sorted by code: DF before WRC
        assert_eq!(species[0].code, "DF");
        assert_eq!(species[1].code, "WRC");
    }

    #[test]
    fn test_species_list_deduplicates() {
        let inv = sample_inventory();
        let species = inv.species_list();
        // DF appears in both plots but should be listed once
        let df_count = species.iter().filter(|s| s.code == "DF").count();
        assert_eq!(df_count, 1);
    }

    #[test]
    fn test_species_list_includes_dead_trees() {
        let inv = sample_inventory();
        let species = inv.species_list();
        // The dead DF tree should still contribute to species list
        assert!(species.iter().any(|s| s.code == "DF"));
    }

    #[test]
    fn test_species_list_empty() {
        let inv = ForestInventory::new("Empty");
        assert!(inv.species_list().is_empty());
    }

    #[test]
    fn test_mean_tpa() {
        let inv = sample_inventory();
        let tpa = inv.mean_tpa();
        assert!(tpa > 0.0);
        // Plot 1: 5.0 + 5.0 = 10.0 TPA (two live trees)
        // Plot 2: 5.0 TPA (one live tree, one dead excluded)
        // Mean: (10.0 + 5.0) / 2 = 7.5
        assert!((tpa - 7.5).abs() < 0.001);
    }

    #[test]
    fn test_mean_tpa_empty() {
        let inv = ForestInventory::new("Empty");
        assert_eq!(inv.mean_tpa(), 0.0);
    }

    #[test]
    fn test_mean_basal_area() {
        let inv = sample_inventory();
        let ba = inv.mean_basal_area();
        assert!(ba > 0.0);
    }

    #[test]
    fn test_mean_basal_area_empty() {
        let inv = ForestInventory::new("Empty");
        assert_eq!(inv.mean_basal_area(), 0.0);
    }

    #[test]
    fn test_mean_volume_cuft() {
        let inv = sample_inventory();
        let vol = inv.mean_volume_cuft();
        assert!(vol > 0.0);
    }

    #[test]
    fn test_mean_volume_cuft_empty() {
        let inv = ForestInventory::new("Empty");
        assert_eq!(inv.mean_volume_cuft(), 0.0);
    }

    #[test]
    fn test_mean_volume_bdft() {
        let inv = sample_inventory();
        let vol = inv.mean_volume_bdft();
        assert!(vol > 0.0);
    }

    #[test]
    fn test_mean_volume_bdft_empty() {
        let inv = ForestInventory::new("Empty");
        assert_eq!(inv.mean_volume_bdft(), 0.0);
    }

    #[test]
    fn test_inventory_json_roundtrip() {
        let inv = sample_inventory();
        let json = serde_json::to_string(&inv).unwrap();
        let deserialized: ForestInventory = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, inv.name);
        assert_eq!(deserialized.num_plots(), inv.num_plots());
        assert_eq!(deserialized.num_trees(), inv.num_trees());
    }

    #[test]
    fn test_single_plot_means_equal_plot_values() {
        let df = make_species("DF", "Douglas Fir");
        let mut inv = ForestInventory::new("Single Plot");
        let plot = make_plot_with_trees(1, vec![make_tree(1, df.clone(), 14.0, TreeStatus::Live)]);
        let plot_tpa = plot.trees_per_acre();
        let plot_ba = plot.basal_area_per_acre();
        inv.plots.push(plot);

        assert!((inv.mean_tpa() - plot_tpa).abs() < 0.001);
        assert!((inv.mean_basal_area() - plot_ba).abs() < 0.001);
    }
}
