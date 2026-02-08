use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::models::{ForestInventory, Species};

/// Per-species composition data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeciesComposition {
    pub species: Species,
    pub tpa: f64,
    pub basal_area: f64,
    pub percent_tpa: f64,
    pub percent_basal_area: f64,
    pub mean_dbh: f64,
    pub mean_height: Option<f64>,
}

/// Overall stand-level metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandMetrics {
    pub total_tpa: f64,
    pub total_basal_area: f64,
    pub total_volume_cuft: f64,
    pub total_volume_bdft: f64,
    pub quadratic_mean_diameter: f64,
    pub mean_height: Option<f64>,
    pub num_species: usize,
    pub species_composition: Vec<SpeciesComposition>,
}

/// Compute stand-level metrics from a forest inventory.
pub fn compute_stand_metrics(inventory: &ForestInventory) -> StandMetrics {
    let num_plots = inventory.num_plots() as f64;
    if num_plots == 0.0 {
        return StandMetrics {
            total_tpa: 0.0,
            total_basal_area: 0.0,
            total_volume_cuft: 0.0,
            total_volume_bdft: 0.0,
            quadratic_mean_diameter: 0.0,
            mean_height: None,
            num_species: 0,
            species_composition: Vec::new(),
        };
    }

    let total_tpa = inventory.mean_tpa();
    let total_ba = inventory.mean_basal_area();
    let total_vol_cuft = inventory.mean_volume_cuft();
    let total_vol_bdft = inventory.mean_volume_bdft();

    // QMD across all plots
    let sum_qmd: f64 = inventory
        .plots
        .iter()
        .map(|p| p.quadratic_mean_diameter())
        .sum();
    let qmd = sum_qmd / num_plots;

    // Mean height of all live trees with height measurements
    let (height_sum, height_count) = inventory
        .plots
        .iter()
        .flat_map(|p| p.live_trees())
        .filter_map(|t| t.height)
        .fold((0.0, 0usize), |(sum, count), h| (sum + h, count + 1));
    let mean_height = if height_count > 0 {
        Some(height_sum / height_count as f64)
    } else {
        None
    };

    // Species composition
    // (species, tpa_sum, ba_sum, weighted_dbh_sum, tree_count, height_sum, height_count)
    type SpeciesAccum = (Species, f64, f64, f64, usize, f64, usize);
    let mut species_data: HashMap<String, SpeciesAccum> = HashMap::new();

    for plot in &inventory.plots {
        for tree in plot.live_trees() {
            let entry = species_data
                .entry(tree.species.code.clone())
                .or_insert_with(|| {
                    (tree.species.clone(), 0.0, 0.0, 0.0, 0, 0.0, 0)
                });
            entry.1 += tree.expansion_factor; // TPA sum
            entry.2 += tree.basal_area_per_acre(); // BA sum
            entry.3 += tree.dbh * tree.expansion_factor; // weighted DBH sum
            entry.4 += 1; // tree count
            if let Some(h) = tree.height {
                entry.5 += h;
                entry.6 += 1;
            }
        }
    }

    let mut species_comp: Vec<SpeciesComposition> = species_data
        .into_values()
        .map(|(species, tpa_sum, ba_sum, dbh_sum, _count, h_sum, h_count)| {
            let tpa = tpa_sum / num_plots;
            let ba = ba_sum / num_plots;
            let mean_dbh = if tpa_sum > 0.0 {
                dbh_sum / tpa_sum
            } else {
                0.0
            };
            let mean_h = if h_count > 0 {
                Some(h_sum / h_count as f64)
            } else {
                None
            };
            SpeciesComposition {
                species,
                tpa,
                basal_area: ba,
                percent_tpa: if total_tpa > 0.0 {
                    (tpa / total_tpa) * 100.0
                } else {
                    0.0
                },
                percent_basal_area: if total_ba > 0.0 {
                    (ba / total_ba) * 100.0
                } else {
                    0.0
                },
                mean_dbh,
                mean_height: mean_h,
            }
        })
        .collect();

    species_comp.sort_by(|a, b| b.basal_area.partial_cmp(&a.basal_area).unwrap());

    StandMetrics {
        total_tpa,
        total_basal_area: total_ba,
        total_volume_cuft: total_vol_cuft,
        total_volume_bdft: total_vol_bdft,
        quadratic_mean_diameter: qmd,
        mean_height,
        num_species: species_comp.len(),
        species_composition: species_comp,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Plot, Tree, TreeStatus};

    fn make_species(code: &str, name: &str) -> Species {
        Species {
            common_name: name.to_string(),
            code: code.to_string(),
        }
    }

    fn make_tree(plot_id: u32, species: Species, dbh: f64, height: Option<f64>, status: TreeStatus) -> Tree {
        Tree {
            tree_id: 1,
            plot_id,
            species,
            dbh,
            height,
            crown_ratio: Some(0.5),
            status,
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

    fn sample_inventory() -> ForestInventory {
        let df = make_species("DF", "Douglas Fir");
        let wrc = make_species("WRC", "Western Red Cedar");

        let mut inv = ForestInventory::new("Metrics Test");
        inv.plots.push(make_plot(1, vec![
            make_tree(1, df.clone(), 16.0, Some(100.0), TreeStatus::Live),
            make_tree(1, wrc.clone(), 12.0, Some(80.0), TreeStatus::Live),
            make_tree(1, df.clone(), 10.0, Some(50.0), TreeStatus::Dead),
        ]));
        inv.plots.push(make_plot(2, vec![
            make_tree(2, df.clone(), 18.0, Some(110.0), TreeStatus::Live),
            make_tree(2, wrc.clone(), 14.0, Some(90.0), TreeStatus::Live),
        ]));
        inv
    }

    #[test]
    fn test_empty_inventory_metrics() {
        let inv = ForestInventory::new("Empty");
        let metrics = compute_stand_metrics(&inv);
        assert_eq!(metrics.total_tpa, 0.0);
        assert_eq!(metrics.total_basal_area, 0.0);
        assert_eq!(metrics.total_volume_cuft, 0.0);
        assert_eq!(metrics.total_volume_bdft, 0.0);
        assert_eq!(metrics.quadratic_mean_diameter, 0.0);
        assert!(metrics.mean_height.is_none());
        assert_eq!(metrics.num_species, 0);
        assert!(metrics.species_composition.is_empty());
    }

    #[test]
    fn test_metrics_positive_values() {
        let inv = sample_inventory();
        let metrics = compute_stand_metrics(&inv);
        assert!(metrics.total_tpa > 0.0);
        assert!(metrics.total_basal_area > 0.0);
        assert!(metrics.total_volume_cuft > 0.0);
        assert!(metrics.total_volume_bdft > 0.0);
        assert!(metrics.quadratic_mean_diameter > 0.0);
    }

    #[test]
    fn test_species_count() {
        let inv = sample_inventory();
        let metrics = compute_stand_metrics(&inv);
        // Only live trees: DF and WRC (dead DF excluded from species comp)
        assert_eq!(metrics.num_species, 2);
    }

    #[test]
    fn test_species_composition_not_empty() {
        let inv = sample_inventory();
        let metrics = compute_stand_metrics(&inv);
        assert_eq!(metrics.species_composition.len(), 2);
    }

    #[test]
    fn test_species_percentages_sum_to_100() {
        let inv = sample_inventory();
        let metrics = compute_stand_metrics(&inv);
        let tpa_pct_sum: f64 = metrics.species_composition.iter().map(|s| s.percent_tpa).sum();
        let ba_pct_sum: f64 = metrics.species_composition.iter().map(|s| s.percent_basal_area).sum();
        assert!((tpa_pct_sum - 100.0).abs() < 0.1);
        assert!((ba_pct_sum - 100.0).abs() < 0.1);
    }

    #[test]
    fn test_species_sorted_by_basal_area_desc() {
        let inv = sample_inventory();
        let metrics = compute_stand_metrics(&inv);
        for i in 1..metrics.species_composition.len() {
            assert!(
                metrics.species_composition[i - 1].basal_area
                    >= metrics.species_composition[i].basal_area
            );
        }
    }

    #[test]
    fn test_mean_height_present() {
        let inv = sample_inventory();
        let metrics = compute_stand_metrics(&inv);
        assert!(metrics.mean_height.is_some());
    }

    #[test]
    fn test_mean_height_none_when_no_heights() {
        let df = make_species("DF", "Douglas Fir");
        let mut inv = ForestInventory::new("No Heights");
        inv.plots.push(make_plot(1, vec![
            make_tree(1, df, 12.0, None, TreeStatus::Live),
        ]));
        let metrics = compute_stand_metrics(&inv);
        assert!(metrics.mean_height.is_none());
    }

    #[test]
    fn test_qmd_reasonable_range() {
        let inv = sample_inventory();
        let metrics = compute_stand_metrics(&inv);
        // QMD should be between the smallest and largest live DBH
        assert!(metrics.quadratic_mean_diameter >= 12.0);
        assert!(metrics.quadratic_mean_diameter <= 18.0);
    }

    #[test]
    fn test_species_mean_dbh() {
        let inv = sample_inventory();
        let metrics = compute_stand_metrics(&inv);
        for sp in &metrics.species_composition {
            assert!(sp.mean_dbh > 0.0);
        }
    }

    #[test]
    fn test_species_mean_height() {
        let inv = sample_inventory();
        let metrics = compute_stand_metrics(&inv);
        for sp in &metrics.species_composition {
            assert!(sp.mean_height.is_some());
            assert!(sp.mean_height.unwrap() > 0.0);
        }
    }

    #[test]
    fn test_single_species_inventory() {
        let df = make_species("DF", "Douglas Fir");
        let mut inv = ForestInventory::new("Single Species");
        inv.plots.push(make_plot(1, vec![
            make_tree(1, df.clone(), 14.0, Some(90.0), TreeStatus::Live),
            make_tree(1, df.clone(), 16.0, Some(100.0), TreeStatus::Live),
        ]));
        let metrics = compute_stand_metrics(&inv);
        assert_eq!(metrics.num_species, 1);
        assert!((metrics.species_composition[0].percent_tpa - 100.0).abs() < 0.1);
        assert!((metrics.species_composition[0].percent_basal_area - 100.0).abs() < 0.1);
    }

    #[test]
    fn test_all_dead_trees_metrics() {
        let df = make_species("DF", "Douglas Fir");
        let mut inv = ForestInventory::new("All Dead");
        inv.plots.push(make_plot(1, vec![
            make_tree(1, df.clone(), 14.0, Some(90.0), TreeStatus::Dead),
            make_tree(1, df.clone(), 16.0, Some(100.0), TreeStatus::Dead),
        ]));
        let metrics = compute_stand_metrics(&inv);
        assert_eq!(metrics.total_tpa, 0.0);
        assert_eq!(metrics.total_basal_area, 0.0);
        assert_eq!(metrics.num_species, 0);
    }

    #[test]
    fn test_metrics_json_roundtrip() {
        let inv = sample_inventory();
        let metrics = compute_stand_metrics(&inv);
        let json = serde_json::to_string(&metrics).unwrap();
        let deserialized: StandMetrics = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.num_species, metrics.num_species);
        assert!((deserialized.total_tpa - metrics.total_tpa).abs() < 0.001);
    }
}
