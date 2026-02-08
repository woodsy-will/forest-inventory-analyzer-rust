use serde::{Deserialize, Serialize};

use crate::models::ForestInventory;

/// A single diameter class in the distribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiameterClass {
    /// Lower bound of the class (inclusive)
    pub lower: f64,
    /// Upper bound of the class (exclusive)
    pub upper: f64,
    /// Midpoint of the class
    pub midpoint: f64,
    /// Trees per acre in this class
    pub tpa: f64,
    /// Basal area per acre in this class
    pub basal_area: f64,
    /// Number of measured trees in this class
    pub tree_count: usize,
}

/// Diameter distribution for the stand.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiameterDistribution {
    /// Width of each diameter class in inches
    pub class_width: f64,
    /// The diameter classes
    pub classes: Vec<DiameterClass>,
}

impl DiameterDistribution {
    /// Build a diameter distribution from the inventory.
    ///
    /// # Arguments
    /// * `inventory` - The forest inventory data
    /// * `class_width` - Width of each diameter class in inches (commonly 2)
    pub fn from_inventory(inventory: &ForestInventory, class_width: f64) -> Self {
        let num_plots = inventory.num_plots() as f64;
        if num_plots == 0.0 {
            return DiameterDistribution {
                class_width,
                classes: Vec::new(),
            };
        }

        // Find DBH range
        let all_live_dbh: Vec<(f64, f64)> = inventory
            .plots
            .iter()
            .flat_map(|p| {
                p.live_trees()
                    .into_iter()
                    .map(|t| (t.dbh, t.expansion_factor))
            })
            .collect();

        if all_live_dbh.is_empty() {
            return DiameterDistribution {
                class_width,
                classes: Vec::new(),
            };
        }

        let min_dbh = all_live_dbh
            .iter()
            .map(|(d, _)| *d)
            .fold(f64::INFINITY, f64::min);
        let max_dbh = all_live_dbh
            .iter()
            .map(|(d, _)| *d)
            .fold(f64::NEG_INFINITY, f64::max);

        // Build classes starting from the lower bound
        let start = (min_dbh / class_width).floor() * class_width;
        let end = ((max_dbh / class_width).floor() + 1.0) * class_width;

        let mut classes = Vec::new();
        let mut lower = start;
        while lower < end {
            let upper = lower + class_width;
            let midpoint = lower + class_width / 2.0;

            let mut tpa_sum = 0.0;
            let mut ba_sum = 0.0;
            let mut count = 0usize;

            for plot in &inventory.plots {
                for tree in plot.live_trees() {
                    if tree.dbh >= lower && tree.dbh < upper {
                        tpa_sum += tree.expansion_factor;
                        ba_sum += tree.basal_area_per_acre();
                        count += 1;
                    }
                }
            }

            if count > 0 {
                classes.push(DiameterClass {
                    lower,
                    upper,
                    midpoint,
                    tpa: tpa_sum / num_plots,
                    basal_area: ba_sum / num_plots,
                    tree_count: count,
                });
            }

            lower = upper;
        }

        DiameterDistribution {
            class_width,
            classes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Plot, Species, Tree, TreeStatus};

    fn make_tree(plot_id: u32, dbh: f64, ef: f64) -> Tree {
        Tree {
            tree_id: 1,
            plot_id,
            species: Species {
                common_name: "Douglas Fir".to_string(),
                code: "DF".to_string(),
            },
            dbh,
            height: Some(80.0),
            crown_ratio: Some(0.5),
            status: TreeStatus::Live,
            expansion_factor: ef,
            age: None,
            defect: None,
        }
    }

    fn make_dead_tree(plot_id: u32, dbh: f64) -> Tree {
        Tree {
            tree_id: 2,
            plot_id,
            species: Species {
                common_name: "Douglas Fir".to_string(),
                code: "DF".to_string(),
            },
            dbh,
            height: Some(60.0),
            crown_ratio: None,
            status: TreeStatus::Dead,
            expansion_factor: 5.0,
            age: None,
            defect: None,
        }
    }

    fn make_plot(plot_id: u32, trees: Vec<Tree>) -> Plot {
        Plot {
            plot_id,
            plot_size_acres: 0.2,
            slope_percent: None,
            aspect_degrees: None,
            elevation_ft: None,
            trees,
        }
    }

    #[test]
    fn test_empty_inventory() {
        let inv = ForestInventory::new("Empty");
        let dist = DiameterDistribution::from_inventory(&inv, 2.0);
        assert!(dist.classes.is_empty());
        assert_eq!(dist.class_width, 2.0);
    }

    #[test]
    fn test_all_dead_trees() {
        let mut inv = ForestInventory::new("Dead");
        inv.plots.push(make_plot(1, vec![
            make_dead_tree(1, 12.0),
            make_dead_tree(1, 16.0),
        ]));
        let dist = DiameterDistribution::from_inventory(&inv, 2.0);
        assert!(dist.classes.is_empty());
    }

    #[test]
    fn test_single_tree_single_class() {
        let mut inv = ForestInventory::new("Single");
        inv.plots.push(make_plot(1, vec![
            make_tree(1, 13.0, 5.0),
        ]));
        let dist = DiameterDistribution::from_inventory(&inv, 2.0);
        assert_eq!(dist.classes.len(), 1);
        assert_eq!(dist.classes[0].tree_count, 1);
        assert!((dist.classes[0].tpa - 5.0).abs() < 0.001);
        assert!(dist.classes[0].lower <= 13.0);
        assert!(dist.classes[0].upper > 13.0);
    }

    #[test]
    fn test_class_width_2_inch() {
        let mut inv = ForestInventory::new("Test");
        inv.plots.push(make_plot(1, vec![
            make_tree(1, 10.0, 5.0),
            make_tree(1, 11.0, 5.0),
            make_tree(1, 14.0, 5.0),
            make_tree(1, 15.0, 5.0),
        ]));
        let dist = DiameterDistribution::from_inventory(&inv, 2.0);
        // 10-12 class: trees at 10" and 11"
        // 14-16 class: trees at 14" and 15"
        assert_eq!(dist.classes.len(), 2);
        assert_eq!(dist.classes[0].tree_count, 2);
        assert_eq!(dist.classes[1].tree_count, 2);
    }

    #[test]
    fn test_class_width_1_inch() {
        let mut inv = ForestInventory::new("Test");
        inv.plots.push(make_plot(1, vec![
            make_tree(1, 10.0, 5.0),
            make_tree(1, 11.0, 5.0),
        ]));
        let dist = DiameterDistribution::from_inventory(&inv, 1.0);
        assert_eq!(dist.classes.len(), 2);
    }

    #[test]
    fn test_midpoint_calculation() {
        let mut inv = ForestInventory::new("Test");
        inv.plots.push(make_plot(1, vec![
            make_tree(1, 13.0, 5.0),
        ]));
        let dist = DiameterDistribution::from_inventory(&inv, 2.0);
        let class = &dist.classes[0];
        assert!((class.midpoint - (class.lower + class.upper) / 2.0).abs() < 0.001);
    }

    #[test]
    fn test_tpa_averaged_across_plots() {
        let mut inv = ForestInventory::new("Multi Plot");
        // Two plots, each with one 12" tree at EF=5
        inv.plots.push(make_plot(1, vec![make_tree(1, 12.0, 5.0)]));
        inv.plots.push(make_plot(2, vec![make_tree(2, 12.0, 5.0)]));
        let dist = DiameterDistribution::from_inventory(&inv, 2.0);
        assert_eq!(dist.classes.len(), 1);
        // Total TPA sum = 5 + 5 = 10, divided by 2 plots = 5.0
        assert!((dist.classes[0].tpa - 5.0).abs() < 0.001);
        assert_eq!(dist.classes[0].tree_count, 2);
    }

    #[test]
    fn test_basal_area_in_classes() {
        let mut inv = ForestInventory::new("BA Test");
        inv.plots.push(make_plot(1, vec![make_tree(1, 12.0, 5.0)]));
        let dist = DiameterDistribution::from_inventory(&inv, 2.0);
        assert!(dist.classes[0].basal_area > 0.0);
    }

    #[test]
    fn test_excludes_dead_trees() {
        let mut inv = ForestInventory::new("Mix");
        inv.plots.push(make_plot(1, vec![
            make_tree(1, 12.0, 5.0),
            make_dead_tree(1, 16.0),
        ]));
        let dist = DiameterDistribution::from_inventory(&inv, 2.0);
        // Only the live 12" tree should be in the distribution
        assert_eq!(dist.classes.len(), 1);
        assert_eq!(dist.classes[0].tree_count, 1);
    }

    #[test]
    fn test_wide_range_of_diameters() {
        let mut inv = ForestInventory::new("Wide Range");
        inv.plots.push(make_plot(1, vec![
            make_tree(1, 4.0, 10.0),
            make_tree(1, 12.0, 5.0),
            make_tree(1, 24.0, 3.0),
            make_tree(1, 36.0, 1.0),
        ]));
        let dist = DiameterDistribution::from_inventory(&inv, 2.0);
        assert!(dist.classes.len() >= 4);
        // Classes should be ordered by diameter
        for i in 1..dist.classes.len() {
            assert!(dist.classes[i].lower >= dist.classes[i - 1].lower);
        }
    }

    #[test]
    fn test_distribution_json_roundtrip() {
        let mut inv = ForestInventory::new("JSON Test");
        inv.plots.push(make_plot(1, vec![
            make_tree(1, 12.0, 5.0),
            make_tree(1, 16.0, 3.0),
        ]));
        let dist = DiameterDistribution::from_inventory(&inv, 2.0);
        let json = serde_json::to_string(&dist).unwrap();
        let deserialized: DiameterDistribution = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.classes.len(), dist.classes.len());
        assert_eq!(deserialized.class_width, dist.class_width);
    }
}
