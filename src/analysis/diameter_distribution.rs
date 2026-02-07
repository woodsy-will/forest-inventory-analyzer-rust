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
