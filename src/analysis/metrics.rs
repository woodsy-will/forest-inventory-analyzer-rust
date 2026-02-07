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
