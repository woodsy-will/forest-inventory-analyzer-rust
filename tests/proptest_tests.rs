use proptest::prelude::*;

use forest_inventory_analyzer::{
    analysis::{
        compute_stand_metrics, project_growth, DiameterDistribution, GrowthModel,
        SamplingStatistics,
    },
    models::{ForestInventory, Plot, Species, Tree, TreeStatus},
};

/// Strategy to generate a valid Species.
fn arb_species() -> impl Strategy<Value = Species> {
    prop_oneof![
        Just(Species {
            common_name: "Douglas Fir".to_string(),
            code: "DF".to_string(),
        }),
        Just(Species {
            common_name: "Western Red Cedar".to_string(),
            code: "WRC".to_string(),
        }),
        Just(Species {
            common_name: "Western Hemlock".to_string(),
            code: "WH".to_string(),
        }),
        Just(Species {
            common_name: "Ponderosa Pine".to_string(),
            code: "PP".to_string(),
        }),
    ]
}

/// Strategy to generate a valid Tree with positive DBH, optional positive height, and positive EF.
fn arb_tree(plot_id: u32) -> impl Strategy<Value = Tree> {
    (
        arb_species(),
        1.0f64..80.0,              // dbh: 1 to 80 inches
        prop::option::of(10.0f64..250.0), // height: 10 to 250 feet
        1.0f64..20.0,              // expansion_factor
    )
        .prop_map(move |(species, dbh, height, ef)| Tree {
            tree_id: 1,
            plot_id,
            species,
            dbh,
            height,
            crown_ratio: Some(0.5),
            status: TreeStatus::Live,
            expansion_factor: ef,
            age: None,
            defect: None,
        })
}

/// Strategy to generate a valid Plot with 1-5 live trees.
fn arb_plot(plot_id: u32) -> impl Strategy<Value = Plot> {
    prop::collection::vec(arb_tree(plot_id), 1..=5).prop_map(move |trees| Plot {
        plot_id,
        plot_size_acres: 0.2,
        slope_percent: None,
        aspect_degrees: None,
        elevation_ft: None,
        trees,
        stand_id: None,
    })
}

/// Strategy to generate a ForestInventory with 2-5 plots (enough for statistics).
fn arb_inventory() -> impl Strategy<Value = ForestInventory> {
    (2u32..=5).prop_flat_map(|num_plots| {
        let plots: Vec<_> = (1..=num_plots).map(|id| arb_plot(id)).collect();
        plots.prop_map(|plots| {
            let mut inv = ForestInventory::new("PropTest Inventory");
            inv.plots = plots;
            inv
        })
    })
}

proptest! {
    // --- Tree basal area is always non-negative for any positive DBH ---
    #[test]
    fn basal_area_non_negative_for_positive_dbh(dbh in 0.001f64..1000.0) {
        let tree = Tree {
            tree_id: 1,
            plot_id: 1,
            species: Species { common_name: "Douglas Fir".into(), code: "DF".into() },
            dbh,
            height: Some(100.0),
            crown_ratio: Some(0.5),
            status: TreeStatus::Live,
            expansion_factor: 5.0,
            age: None,
            defect: None,
        };
        let ba = tree.basal_area_sqft();
        prop_assert!(ba >= 0.0, "basal area was negative: {} for dbh {}", ba, dbh);
        prop_assert!(ba > 0.0, "basal area was zero for positive dbh {}", dbh);
    }

    // --- Volume is always non-negative when height and DBH are positive ---
    #[test]
    fn volume_non_negative_for_positive_inputs(
        dbh in 0.1f64..100.0,
        height in 1.0f64..300.0,
    ) {
        let tree = Tree {
            tree_id: 1,
            plot_id: 1,
            species: Species { common_name: "Douglas Fir".into(), code: "DF".into() },
            dbh,
            height: Some(height),
            crown_ratio: None,
            status: TreeStatus::Live,
            expansion_factor: 5.0,
            age: None,
            defect: None,
        };
        if let Some(vol) = tree.volume_cuft() {
            prop_assert!(vol >= 0.0, "cubic ft volume was negative: {} for dbh={}, ht={}", vol, dbh, height);
        }
        if let Some(vol) = tree.volume_bdft() {
            prop_assert!(vol >= 0.0, "board ft volume was negative: {} for dbh={}, ht={}", vol, dbh, height);
        }
    }

    // --- Confidence interval: lower <= mean <= upper for any valid data ---
    #[test]
    fn confidence_interval_ordering(ref inv in arb_inventory()) {
        let stats = SamplingStatistics::compute(inv, 0.95);
        if let Ok(stats) = stats {
            prop_assert!(
                stats.tpa.lower <= stats.tpa.mean,
                "TPA: lower {} > mean {}", stats.tpa.lower, stats.tpa.mean
            );
            prop_assert!(
                stats.tpa.mean <= stats.tpa.upper,
                "TPA: mean {} > upper {}", stats.tpa.mean, stats.tpa.upper
            );
            prop_assert!(
                stats.basal_area.lower <= stats.basal_area.mean,
                "BA: lower {} > mean {}", stats.basal_area.lower, stats.basal_area.mean
            );
            prop_assert!(
                stats.basal_area.mean <= stats.basal_area.upper,
                "BA: mean {} > upper {}", stats.basal_area.mean, stats.basal_area.upper
            );
            prop_assert!(
                stats.volume_cuft.lower <= stats.volume_cuft.mean,
                "Vol cuft: lower {} > mean {}", stats.volume_cuft.lower, stats.volume_cuft.mean
            );
            prop_assert!(
                stats.volume_cuft.mean <= stats.volume_cuft.upper,
                "Vol cuft: mean {} > upper {}", stats.volume_cuft.mean, stats.volume_cuft.upper
            );
        }
    }

    // --- Species percentages in StandMetrics always sum to ~100% ---
    #[test]
    fn species_percentages_sum_to_100(ref inv in arb_inventory()) {
        let metrics = compute_stand_metrics(inv);
        if !metrics.species_composition.is_empty() {
            let tpa_pct_sum: f64 = metrics.species_composition.iter().map(|s| s.percent_tpa).sum();
            let ba_pct_sum: f64 = metrics.species_composition.iter().map(|s| s.percent_basal_area).sum();

            prop_assert!(
                (tpa_pct_sum - 100.0).abs() < 1.0,
                "TPA percentages sum to {} (expected ~100)", tpa_pct_sum
            );
            prop_assert!(
                (ba_pct_sum - 100.0).abs() < 1.0,
                "BA percentages sum to {} (expected ~100)", ba_pct_sum
            );
        }
    }

    // --- DiameterDistribution classes are always sorted by lower bound ---
    #[test]
    fn diameter_distribution_classes_sorted(ref inv in arb_inventory()) {
        let dist = DiameterDistribution::from_inventory(inv, 2.0);
        for i in 1..dist.classes.len() {
            prop_assert!(
                dist.classes[i].lower >= dist.classes[i - 1].lower,
                "Classes not sorted: class {} lower={} < class {} lower={}",
                i, dist.classes[i].lower, i - 1, dist.classes[i - 1].lower
            );
        }
    }

    // --- Growth projections never produce negative TPA, BA, or volume ---
    #[test]
    fn growth_projections_non_negative(ref inv in arb_inventory()) {
        let models = vec![
            GrowthModel::Exponential {
                annual_rate: 0.03,
                mortality_rate: 0.005,
            },
            GrowthModel::Logistic {
                annual_rate: 0.03,
                carrying_capacity: 300.0,
                mortality_rate: 0.005,
            },
            GrowthModel::Linear {
                annual_increment: 2.0,
                mortality_rate: 0.5,
            },
        ];
        for model in &models {
            if let Ok(projections) = project_growth(inv, model, 30) {
                for p in &projections {
                    prop_assert!(p.tpa >= 0.0, "Negative TPA {} at year {}", p.tpa, p.year);
                    prop_assert!(p.basal_area >= 0.0, "Negative BA {} at year {}", p.basal_area, p.year);
                    prop_assert!(p.volume_cuft >= 0.0, "Negative vol cuft {} at year {}", p.volume_cuft, p.year);
                    prop_assert!(p.volume_bdft >= 0.0, "Negative vol bdft {} at year {}", p.volume_bdft, p.year);
                }
            }
        }
    }
}
