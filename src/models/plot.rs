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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Species, TreeStatus};

    fn make_tree(tree_id: u32, dbh: f64, height: Option<f64>, status: TreeStatus, ef: f64) -> Tree {
        Tree {
            tree_id,
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
            age: None,
            defect: None,
        }
    }

    fn make_plot(trees: Vec<Tree>) -> Plot {
        Plot {
            plot_id: 1,
            plot_size_acres: 0.2,
            slope_percent: Some(15.0),
            aspect_degrees: Some(180.0),
            elevation_ft: Some(3000.0),
            trees,
        }
    }

    #[test]
    fn test_live_trees_filters_correctly() {
        let plot = make_plot(vec![
            make_tree(1, 12.0, Some(80.0), TreeStatus::Live, 5.0),
            make_tree(2, 10.0, Some(60.0), TreeStatus::Dead, 5.0),
            make_tree(3, 14.0, Some(90.0), TreeStatus::Live, 5.0),
            make_tree(4, 8.0, Some(40.0), TreeStatus::Cut, 5.0),
        ]);
        let live = plot.live_trees();
        assert_eq!(live.len(), 2);
        assert_eq!(live[0].tree_id, 1);
        assert_eq!(live[1].tree_id, 3);
    }

    #[test]
    fn test_live_trees_empty_plot() {
        let plot = make_plot(vec![]);
        assert!(plot.live_trees().is_empty());
    }

    #[test]
    fn test_live_trees_all_dead() {
        let plot = make_plot(vec![
            make_tree(1, 12.0, Some(80.0), TreeStatus::Dead, 5.0),
            make_tree(2, 10.0, Some(60.0), TreeStatus::Dead, 5.0),
        ]);
        assert!(plot.live_trees().is_empty());
    }

    #[test]
    fn test_trees_per_acre() {
        let plot = make_plot(vec![
            make_tree(1, 12.0, Some(80.0), TreeStatus::Live, 5.0),
            make_tree(2, 10.0, Some(60.0), TreeStatus::Live, 3.0),
            make_tree(3, 8.0, Some(40.0), TreeStatus::Dead, 5.0),
        ]);
        // Only live trees: 5.0 + 3.0 = 8.0
        assert!((plot.trees_per_acre() - 8.0).abs() < 0.001);
    }

    #[test]
    fn test_trees_per_acre_empty() {
        let plot = make_plot(vec![]);
        assert_eq!(plot.trees_per_acre(), 0.0);
    }

    #[test]
    fn test_basal_area_per_acre() {
        let plot = make_plot(vec![
            make_tree(1, 12.0, Some(80.0), TreeStatus::Live, 5.0),
        ]);
        let expected_ba = std::f64::consts::PI * 36.0 / 144.0 * 5.0;
        assert!((plot.basal_area_per_acre() - expected_ba).abs() < 0.001);
    }

    #[test]
    fn test_basal_area_excludes_dead() {
        let plot = make_plot(vec![
            make_tree(1, 12.0, Some(80.0), TreeStatus::Live, 5.0),
            make_tree(2, 20.0, Some(100.0), TreeStatus::Dead, 5.0),
        ]);
        // Only the live tree's BA should be counted
        let expected_ba = std::f64::consts::PI * 36.0 / 144.0 * 5.0;
        assert!((plot.basal_area_per_acre() - expected_ba).abs() < 0.001);
    }

    #[test]
    fn test_volume_cuft_per_acre() {
        let plot = make_plot(vec![
            make_tree(1, 16.0, Some(100.0), TreeStatus::Live, 5.0),
        ]);
        let vol = plot.volume_cuft_per_acre();
        // V = 0.002454 * 256 * 100 * 5.0 = 314.1
        assert!((vol - 314.1).abs() < 1.0);
    }

    #[test]
    fn test_volume_cuft_per_acre_no_height() {
        // Trees without height don't contribute to volume
        let plot = make_plot(vec![
            make_tree(1, 16.0, None, TreeStatus::Live, 5.0),
        ]);
        assert_eq!(plot.volume_cuft_per_acre(), 0.0);
    }

    #[test]
    fn test_volume_bdft_per_acre() {
        let plot = make_plot(vec![
            make_tree(1, 16.0, Some(100.0), TreeStatus::Live, 5.0),
        ]);
        let vol = plot.volume_bdft_per_acre();
        assert!(vol > 0.0);
    }

    #[test]
    fn test_volume_bdft_small_trees_zero() {
        let plot = make_plot(vec![
            make_tree(1, 4.0, Some(30.0), TreeStatus::Live, 10.0),
        ]);
        assert_eq!(plot.volume_bdft_per_acre(), 0.0);
    }

    #[test]
    fn test_quadratic_mean_diameter() {
        // Two trees with same DBH and same EF -> QMD should equal that DBH
        let plot = make_plot(vec![
            make_tree(1, 12.0, Some(80.0), TreeStatus::Live, 5.0),
            make_tree(2, 12.0, Some(85.0), TreeStatus::Live, 5.0),
        ]);
        assert!((plot.quadratic_mean_diameter() - 12.0).abs() < 0.001);
    }

    #[test]
    fn test_quadratic_mean_diameter_different_sizes() {
        // QMD of 10" and 14" trees (equal EF) = sqrt((100+196)/2) = sqrt(148) ≈ 12.166
        let plot = make_plot(vec![
            make_tree(1, 10.0, Some(70.0), TreeStatus::Live, 5.0),
            make_tree(2, 14.0, Some(90.0), TreeStatus::Live, 5.0),
        ]);
        let expected = ((100.0 + 196.0) / 2.0_f64).sqrt();
        assert!((plot.quadratic_mean_diameter() - expected).abs() < 0.01);
    }

    #[test]
    fn test_quadratic_mean_diameter_empty() {
        let plot = make_plot(vec![]);
        assert_eq!(plot.quadratic_mean_diameter(), 0.0);
    }

    #[test]
    fn test_quadratic_mean_diameter_excludes_dead() {
        let plot = make_plot(vec![
            make_tree(1, 12.0, Some(80.0), TreeStatus::Live, 5.0),
            make_tree(2, 24.0, Some(120.0), TreeStatus::Dead, 5.0),
        ]);
        // QMD should only consider the live 12" tree
        assert!((plot.quadratic_mean_diameter() - 12.0).abs() < 0.001);
    }

    #[test]
    fn test_plot_json_roundtrip() {
        let plot = make_plot(vec![
            make_tree(1, 12.0, Some(80.0), TreeStatus::Live, 5.0),
        ]);
        let json = serde_json::to_string(&plot).unwrap();
        let deserialized: Plot = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.plot_id, plot.plot_id);
        assert_eq!(deserialized.trees.len(), 1);
    }

    #[test]
    fn test_multiple_metrics_consistent() {
        let plot = make_plot(vec![
            make_tree(1, 16.0, Some(100.0), TreeStatus::Live, 5.0),
            make_tree(2, 12.0, Some(80.0), TreeStatus::Live, 3.0),
            make_tree(3, 20.0, Some(110.0), TreeStatus::Dead, 5.0),
        ]);

        assert_eq!(plot.live_trees().len(), 2);
        assert!((plot.trees_per_acre() - 8.0).abs() < 0.001);
        assert!(plot.basal_area_per_acre() > 0.0);
        assert!(plot.volume_cuft_per_acre() > 0.0);
        assert!(plot.volume_bdft_per_acre() > 0.0);
        assert!(plot.quadratic_mean_diameter() > 12.0); // weighted toward larger tree
    }
}
