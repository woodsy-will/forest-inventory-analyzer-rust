use forest_inventory_analyzer::{
    analysis::{
        compute_stand_metrics, project_growth, DiameterDistribution, GrowthModel,
        SamplingStatistics,
    },
    error::ForestError,
    io,
    models::{ForestInventory, Plot, Species, Tree, TreeStatus},
};

fn create_test_inventory() -> ForestInventory {
    let mut inventory = ForestInventory::new("Test Inventory");

    for plot_id in 1..=3 {
        let mut plot = Plot {
            plot_id,
            plot_size_acres: 0.2,
            slope_percent: Some(20.0),
            aspect_degrees: Some(180.0),
            elevation_ft: Some(3000.0),
            trees: Vec::new(),
        };

        // Add trees to each plot with varying expansion factors to create
        // between-plot variability for statistical analysis
        let ef = 4.0 + plot_id as f64; // 5.0, 6.0, 7.0

        plot.trees.push(Tree {
            tree_id: 1,
            plot_id,
            species: Species {
                common_name: "Douglas Fir".to_string(),
                code: "DF".to_string(),
            },
            dbh: 18.0 + plot_id as f64,
            height: Some(100.0 + plot_id as f64 * 5.0),
            crown_ratio: Some(0.45),
            status: TreeStatus::Live,
            expansion_factor: ef,
            age: Some(80),
            defect: None,
        });

        plot.trees.push(Tree {
            tree_id: 2,
            plot_id,
            species: Species {
                common_name: "Western Red Cedar".to_string(),
                code: "WRC".to_string(),
            },
            dbh: 14.0 + plot_id as f64,
            height: Some(85.0 + plot_id as f64 * 3.0),
            crown_ratio: Some(0.50),
            status: TreeStatus::Live,
            expansion_factor: ef,
            age: Some(90),
            defect: Some(0.05),
        });

        plot.trees.push(Tree {
            tree_id: 3,
            plot_id,
            species: Species {
                common_name: "Douglas Fir".to_string(),
                code: "DF".to_string(),
            },
            dbh: 8.0,
            height: Some(50.0),
            crown_ratio: Some(0.65),
            status: TreeStatus::Dead,
            expansion_factor: 5.0,
            age: None,
            defect: None,
        });

        inventory.plots.push(plot);
    }

    inventory
}

// ============================================================================
// Basic inventory tests
// ============================================================================

#[test]
fn test_inventory_basic_stats() {
    let inventory = create_test_inventory();

    assert_eq!(inventory.num_plots(), 3);
    assert_eq!(inventory.num_trees(), 9);
    assert!(inventory.mean_tpa() > 0.0);
    assert!(inventory.mean_basal_area() > 0.0);
}

#[test]
fn test_tree_basal_area() {
    let tree = Tree {
        tree_id: 1,
        plot_id: 1,
        species: Species {
            common_name: "Douglas Fir".to_string(),
            code: "DF".to_string(),
        },
        dbh: 12.0,
        height: Some(80.0),
        crown_ratio: Some(0.50),
        status: TreeStatus::Live,
        expansion_factor: 5.0,
        age: Some(60),
        defect: None,
    };

    let ba = tree.basal_area_sqft();
    // BA = pi * (DBH/2)^2 / 144 = pi * 36 / 144 = 0.7854
    assert!((ba - 0.7854).abs() < 0.001);
}

#[test]
fn test_tree_volume() {
    let tree = Tree {
        tree_id: 1,
        plot_id: 1,
        species: Species {
            common_name: "Douglas Fir".to_string(),
            code: "DF".to_string(),
        },
        dbh: 16.0,
        height: Some(100.0),
        crown_ratio: Some(0.45),
        status: TreeStatus::Live,
        expansion_factor: 5.0,
        age: Some(75),
        defect: None,
    };

    let vol = tree.volume_cuft().unwrap();
    assert!(vol > 0.0);

    let bdft = tree.volume_bdft().unwrap();
    assert!(bdft > 0.0);
}

// ============================================================================
// Stand metrics integration tests
// ============================================================================

#[test]
fn test_stand_metrics() {
    let inventory = create_test_inventory();
    let metrics = compute_stand_metrics(&inventory);

    assert!(metrics.total_tpa > 0.0);
    assert!(metrics.total_basal_area > 0.0);
    assert!(metrics.num_species == 2); // DF and WRC (dead tree excluded from live)
    assert!(!metrics.species_composition.is_empty());
}

#[test]
fn test_stand_metrics_species_percentages() {
    let inventory = create_test_inventory();
    let metrics = compute_stand_metrics(&inventory);

    let tpa_sum: f64 = metrics.species_composition.iter().map(|s| s.percent_tpa).sum();
    let ba_sum: f64 = metrics.species_composition.iter().map(|s| s.percent_basal_area).sum();
    assert!((tpa_sum - 100.0).abs() < 0.1);
    assert!((ba_sum - 100.0).abs() < 0.1);
}

#[test]
fn test_stand_metrics_volumes_positive() {
    let inventory = create_test_inventory();
    let metrics = compute_stand_metrics(&inventory);
    assert!(metrics.total_volume_cuft > 0.0);
    assert!(metrics.total_volume_bdft > 0.0);
}

#[test]
fn test_stand_metrics_qmd_in_range() {
    let inventory = create_test_inventory();
    let metrics = compute_stand_metrics(&inventory);
    // QMD should be between smallest and largest live tree DBH
    assert!(metrics.quadratic_mean_diameter > 10.0);
    assert!(metrics.quadratic_mean_diameter < 25.0);
}

// ============================================================================
// Sampling statistics integration tests
// ============================================================================

#[test]
fn test_sampling_statistics() {
    let inventory = create_test_inventory();
    let stats = SamplingStatistics::compute(&inventory, 0.95).unwrap();

    assert!(stats.tpa.mean > 0.0);
    assert!(stats.tpa.lower < stats.tpa.upper);
    assert_eq!(stats.tpa.sample_size, 3);
    assert!((stats.tpa.confidence_level - 0.95).abs() < 0.001);
}

#[test]
fn test_sampling_statistics_all_metrics_have_ci() {
    let inventory = create_test_inventory();
    let stats = SamplingStatistics::compute(&inventory, 0.95).unwrap();

    // All metrics should have valid confidence intervals
    for ci in &[&stats.tpa, &stats.basal_area, &stats.volume_cuft, &stats.volume_bdft] {
        assert!(ci.lower <= ci.mean);
        assert!(ci.mean <= ci.upper);
        assert!(ci.std_error >= 0.0);
        assert_eq!(ci.sample_size, 3);
    }
}

#[test]
fn test_sampling_statistics_90_vs_95() {
    let inventory = create_test_inventory();
    let stats_90 = SamplingStatistics::compute(&inventory, 0.90).unwrap();
    let stats_95 = SamplingStatistics::compute(&inventory, 0.95).unwrap();

    // 95% CI should be wider than 90%
    let width_90 = stats_90.tpa.upper - stats_90.tpa.lower;
    let width_95 = stats_95.tpa.upper - stats_95.tpa.lower;
    assert!(width_95 > width_90);

    // Means should be the same regardless of confidence level
    assert!((stats_90.tpa.mean - stats_95.tpa.mean).abs() < 0.001);
}

// ============================================================================
// Diameter distribution integration tests
// ============================================================================

#[test]
fn test_diameter_distribution() {
    let inventory = create_test_inventory();
    let dist = DiameterDistribution::from_inventory(&inventory, 2.0);

    assert!(!dist.classes.is_empty());
    assert_eq!(dist.class_width, 2.0);

    // All classes should have positive TPA
    for class in &dist.classes {
        assert!(class.tpa > 0.0);
        assert!(class.basal_area > 0.0);
    }
}

#[test]
fn test_diameter_distribution_class_ordering() {
    let inventory = create_test_inventory();
    let dist = DiameterDistribution::from_inventory(&inventory, 2.0);

    for i in 1..dist.classes.len() {
        assert!(dist.classes[i].lower >= dist.classes[i - 1].upper);
    }
}

#[test]
fn test_diameter_distribution_different_widths() {
    let inventory = create_test_inventory();
    let dist_1 = DiameterDistribution::from_inventory(&inventory, 1.0);
    let dist_2 = DiameterDistribution::from_inventory(&inventory, 2.0);
    let dist_4 = DiameterDistribution::from_inventory(&inventory, 4.0);

    // Narrower classes should produce more (or equal) classes
    assert!(dist_1.classes.len() >= dist_2.classes.len());
    assert!(dist_2.classes.len() >= dist_4.classes.len());
}

// ============================================================================
// Growth projection integration tests
// ============================================================================

#[test]
fn test_growth_projection() {
    let inventory = create_test_inventory();

    let model = GrowthModel::Logistic {
        annual_rate: 0.03,
        carrying_capacity: 300.0,
    };

    let projections = project_growth(&inventory, &model, 10).unwrap();

    assert_eq!(projections.len(), 11); // Year 0 through 10
    assert_eq!(projections[0].year, 0);
    assert_eq!(projections[10].year, 10);

    // Volume should generally increase with logistic growth
    assert!(projections[10].basal_area >= projections[0].basal_area);
}

#[test]
fn test_growth_all_models() {
    let inventory = create_test_inventory();

    let models = vec![
        ("exponential", GrowthModel::Exponential { annual_rate: 0.03 }),
        ("logistic", GrowthModel::Logistic { annual_rate: 0.03, carrying_capacity: 300.0 }),
        ("linear", GrowthModel::Linear { annual_increment: 2.0 }),
    ];

    for (name, model) in &models {
        let projections = project_growth(&inventory, model, 10).unwrap();
        assert_eq!(projections.len(), 11, "Failed for model: {name}");
        assert_eq!(projections[0].year, 0, "Failed for model: {name}");

        // All values should be non-negative
        for p in &projections {
            assert!(p.tpa >= 0.0, "Negative TPA in {name} at year {}", p.year);
            assert!(p.basal_area >= 0.0, "Negative BA in {name} at year {}", p.year);
            assert!(p.volume_cuft >= 0.0, "Negative vol cuft in {name} at year {}", p.year);
            assert!(p.volume_bdft >= 0.0, "Negative vol bdft in {name} at year {}", p.year);
        }
    }
}

#[test]
fn test_growth_empty_inventory_error() {
    let empty = ForestInventory::new("Empty");
    let model = GrowthModel::Exponential { annual_rate: 0.03 };
    assert!(project_growth(&empty, &model, 10).is_err());
}

// ============================================================================
// Species list integration tests
// ============================================================================

#[test]
fn test_species_list() {
    let inventory = create_test_inventory();
    let species = inventory.species_list();

    assert_eq!(species.len(), 2);
    assert!(species.iter().any(|s| s.code == "DF"));
    assert!(species.iter().any(|s| s.code == "WRC"));
}

// ============================================================================
// CSV I/O integration tests
// ============================================================================

#[test]
fn test_csv_roundtrip() {
    let inventory = create_test_inventory();

    let dir = tempfile::tempdir().unwrap();
    let csv_path = dir.path().join("test_output.csv");

    io::write_csv(&inventory, &csv_path).unwrap();
    let loaded = io::read_csv(&csv_path).unwrap();

    assert_eq!(loaded.num_plots(), inventory.num_plots());
    assert_eq!(loaded.num_trees(), inventory.num_trees());
}

#[test]
fn test_csv_preserves_tree_data() {
    let inventory = create_test_inventory();

    let dir = tempfile::tempdir().unwrap();
    let csv_path = dir.path().join("test_preserve.csv");

    io::write_csv(&inventory, &csv_path).unwrap();
    let loaded = io::read_csv(&csv_path).unwrap();

    // Check that tree data survived the roundtrip
    let orig_ba = inventory.mean_basal_area();
    let loaded_ba = loaded.mean_basal_area();
    assert!((orig_ba - loaded_ba).abs() < 0.01);

    let orig_tpa = inventory.mean_tpa();
    let loaded_tpa = loaded.mean_tpa();
    assert!((orig_tpa - loaded_tpa).abs() < 0.01);
}

#[test]
fn test_csv_species_preserved() {
    let inventory = create_test_inventory();

    let dir = tempfile::tempdir().unwrap();
    let csv_path = dir.path().join("test_species.csv");

    io::write_csv(&inventory, &csv_path).unwrap();
    let loaded = io::read_csv(&csv_path).unwrap();

    let orig_species = inventory.species_list();
    let loaded_species = loaded.species_list();
    assert_eq!(orig_species.len(), loaded_species.len());
}

// ============================================================================
// JSON I/O integration tests
// ============================================================================

#[test]
fn test_json_roundtrip() {
    let inventory = create_test_inventory();

    let dir = tempfile::tempdir().unwrap();
    let json_path = dir.path().join("test_output.json");

    io::write_json(&inventory, &json_path, true).unwrap();
    let loaded = io::read_json(&json_path).unwrap();

    assert_eq!(loaded.num_plots(), inventory.num_plots());
    assert_eq!(loaded.num_trees(), inventory.num_trees());
    assert_eq!(loaded.name, inventory.name);
}

#[test]
fn test_json_compact_roundtrip() {
    let inventory = create_test_inventory();

    let dir = tempfile::tempdir().unwrap();
    let json_path = dir.path().join("test_compact.json");

    io::write_json(&inventory, &json_path, false).unwrap();
    let loaded = io::read_json(&json_path).unwrap();

    assert_eq!(loaded.num_plots(), inventory.num_plots());
    assert_eq!(loaded.num_trees(), inventory.num_trees());
}

#[test]
fn test_json_preserves_volumes() {
    let inventory = create_test_inventory();

    let dir = tempfile::tempdir().unwrap();
    let json_path = dir.path().join("test_volumes.json");

    io::write_json(&inventory, &json_path, true).unwrap();
    let loaded = io::read_json(&json_path).unwrap();

    assert!((loaded.mean_volume_cuft() - inventory.mean_volume_cuft()).abs() < 0.001);
    assert!((loaded.mean_volume_bdft() - inventory.mean_volume_bdft()).abs() < 0.001);
}

// ============================================================================
// Excel I/O integration tests
// ============================================================================

#[test]
fn test_excel_roundtrip() {
    let inventory = create_test_inventory();

    let dir = tempfile::tempdir().unwrap();
    let xlsx_path = dir.path().join("test_output.xlsx");

    io::write_excel(&inventory, &xlsx_path).unwrap();
    let loaded = io::read_excel(&xlsx_path).unwrap();

    assert_eq!(loaded.num_plots(), inventory.num_plots());
    assert_eq!(loaded.num_trees(), inventory.num_trees());
}

#[test]
fn test_excel_preserves_metrics() {
    let inventory = create_test_inventory();

    let dir = tempfile::tempdir().unwrap();
    let xlsx_path = dir.path().join("test_metrics.xlsx");

    io::write_excel(&inventory, &xlsx_path).unwrap();
    let loaded = io::read_excel(&xlsx_path).unwrap();

    let orig_ba = inventory.mean_basal_area();
    let loaded_ba = loaded.mean_basal_area();
    assert!((orig_ba - loaded_ba).abs() < 0.1);
}

// ============================================================================
// Format conversion integration tests
// ============================================================================

#[test]
fn test_csv_to_json_conversion() {
    let inventory = create_test_inventory();
    let dir = tempfile::tempdir().unwrap();

    let csv_path = dir.path().join("convert.csv");
    let json_path = dir.path().join("convert.json");

    io::write_csv(&inventory, &csv_path).unwrap();
    let from_csv = io::read_csv(&csv_path).unwrap();
    io::write_json(&from_csv, &json_path, true).unwrap();
    let from_json = io::read_json(&json_path).unwrap();

    assert_eq!(from_json.num_plots(), inventory.num_plots());
    assert_eq!(from_json.num_trees(), inventory.num_trees());
}

#[test]
fn test_json_to_excel_conversion() {
    let inventory = create_test_inventory();
    let dir = tempfile::tempdir().unwrap();

    let json_path = dir.path().join("convert.json");
    let xlsx_path = dir.path().join("convert.xlsx");

    io::write_json(&inventory, &json_path, false).unwrap();
    let from_json = io::read_json(&json_path).unwrap();
    io::write_excel(&from_json, &xlsx_path).unwrap();
    let from_excel = io::read_excel(&xlsx_path).unwrap();

    assert_eq!(from_excel.num_plots(), inventory.num_plots());
    assert_eq!(from_excel.num_trees(), inventory.num_trees());
}

#[test]
fn test_csv_to_excel_to_json_pipeline() {
    let inventory = create_test_inventory();
    let dir = tempfile::tempdir().unwrap();

    // CSV -> Excel -> JSON pipeline
    let csv_path = dir.path().join("pipeline.csv");
    let xlsx_path = dir.path().join("pipeline.xlsx");
    let json_path = dir.path().join("pipeline.json");

    io::write_csv(&inventory, &csv_path).unwrap();
    let from_csv = io::read_csv(&csv_path).unwrap();

    io::write_excel(&from_csv, &xlsx_path).unwrap();
    let from_excel = io::read_excel(&xlsx_path).unwrap();

    io::write_json(&from_excel, &json_path, true).unwrap();
    let final_inv = io::read_json(&json_path).unwrap();

    assert_eq!(final_inv.num_plots(), inventory.num_plots());
    assert_eq!(final_inv.num_trees(), inventory.num_trees());
}

// ============================================================================
// TreeStatus parsing integration tests
// ============================================================================

#[test]
fn test_tree_status_parsing() {
    assert_eq!("live".parse::<TreeStatus>().unwrap(), TreeStatus::Live);
    assert_eq!("L".parse::<TreeStatus>().unwrap(), TreeStatus::Live);
    assert_eq!("dead".parse::<TreeStatus>().unwrap(), TreeStatus::Dead);
    assert_eq!("D".parse::<TreeStatus>().unwrap(), TreeStatus::Dead);
    assert_eq!("cut".parse::<TreeStatus>().unwrap(), TreeStatus::Cut);
    assert!("unknown".parse::<TreeStatus>().is_err());
}

// ============================================================================
// End-to-end workflow tests
// ============================================================================

#[test]
fn test_full_analysis_workflow() {
    // Simulate the full CLI analyze workflow
    let inventory = create_test_inventory();

    // Step 1: Compute stand metrics
    let metrics = compute_stand_metrics(&inventory);
    assert!(metrics.total_tpa > 0.0);
    assert!(metrics.num_species > 0);

    // Step 2: Compute diameter distribution
    let dist = DiameterDistribution::from_inventory(&inventory, 2.0);
    assert!(!dist.classes.is_empty());

    // Step 3: Compute sampling statistics
    let stats = SamplingStatistics::compute(&inventory, 0.95).unwrap();
    assert!(stats.tpa.mean > 0.0);

    // Step 4: Growth projection
    let model = GrowthModel::Logistic {
        annual_rate: 0.03,
        carrying_capacity: 300.0,
    };
    let proj = project_growth(&inventory, &model, 20).unwrap();
    assert_eq!(proj.len(), 21);

    // Verify consistency between metrics and stats
    assert!((metrics.total_tpa - stats.tpa.mean).abs() < 0.01);
    assert!((metrics.total_basal_area - stats.basal_area.mean).abs() < 0.01);
}

#[test]
fn test_analysis_after_format_conversion() {
    let inventory = create_test_inventory();
    let dir = tempfile::tempdir().unwrap();

    // Save to CSV, reload, then analyze
    let csv_path = dir.path().join("analysis.csv");
    io::write_csv(&inventory, &csv_path).unwrap();
    let loaded = io::read_csv(&csv_path).unwrap();

    let orig_metrics = compute_stand_metrics(&inventory);
    let loaded_metrics = compute_stand_metrics(&loaded);

    assert!((orig_metrics.total_tpa - loaded_metrics.total_tpa).abs() < 0.01);
    assert!((orig_metrics.total_basal_area - loaded_metrics.total_basal_area).abs() < 0.01);
    assert_eq!(orig_metrics.num_species, loaded_metrics.num_species);
}

// ============================================================================
// Edge case integration tests
// ============================================================================

#[test]
fn test_single_plot_inventory() {
    let mut inventory = ForestInventory::new("Single Plot");
    inventory.plots.push(Plot {
        plot_id: 1,
        plot_size_acres: 0.2,
        slope_percent: None,
        aspect_degrees: None,
        elevation_ft: None,
        trees: vec![Tree {
            tree_id: 1,
            plot_id: 1,
            species: Species {
                common_name: "Douglas Fir".to_string(),
                code: "DF".to_string(),
            },
            dbh: 14.0,
            height: Some(85.0),
            crown_ratio: Some(0.5),
            status: TreeStatus::Live,
            expansion_factor: 5.0,
            age: Some(60),
            defect: None,
        }],
    });

    let metrics = compute_stand_metrics(&inventory);
    assert!(metrics.total_tpa > 0.0);
    assert_eq!(metrics.num_species, 1);

    // Single plot can't produce valid statistics (need n>=2)
    assert!(SamplingStatistics::compute(&inventory, 0.95).is_err());
}

#[test]
fn test_inventory_all_optional_fields_none() {
    let mut inventory = ForestInventory::new("Minimal");
    inventory.plots.push(Plot {
        plot_id: 1,
        plot_size_acres: 0.2,
        slope_percent: None,
        aspect_degrees: None,
        elevation_ft: None,
        trees: vec![Tree {
            tree_id: 1,
            plot_id: 1,
            species: Species {
                common_name: "Douglas Fir".to_string(),
                code: "DF".to_string(),
            },
            dbh: 14.0,
            height: None,
            crown_ratio: None,
            status: TreeStatus::Live,
            expansion_factor: 5.0,
            age: None,
            defect: None,
        }],
    });

    let metrics = compute_stand_metrics(&inventory);
    assert!(metrics.total_tpa > 0.0);
    assert!(metrics.mean_height.is_none());
    // Volume should be 0 since height is None
    assert_eq!(metrics.total_volume_cuft, 0.0);
    assert_eq!(metrics.total_volume_bdft, 0.0);
}

#[test]
fn test_large_inventory() {
    let mut inventory = ForestInventory::new("Large");

    for plot_id in 1..=50 {
        let mut trees = Vec::new();
        for tree_id in 1..=20 {
            trees.push(Tree {
                tree_id,
                plot_id,
                species: Species {
                    common_name: if tree_id % 3 == 0 { "Western Red Cedar" } else { "Douglas Fir" }.to_string(),
                    code: if tree_id % 3 == 0 { "WRC" } else { "DF" }.to_string(),
                },
                dbh: 8.0 + (tree_id as f64) * 1.5 + (plot_id as f64) * 0.3,
                height: Some(50.0 + tree_id as f64 * 5.0 + plot_id as f64 * 2.0),
                crown_ratio: Some(0.4),
                status: if tree_id % 10 == 0 { TreeStatus::Dead } else { TreeStatus::Live },
                expansion_factor: 4.0 + plot_id as f64 * 0.1,
                age: Some(50 + tree_id),
                defect: None,
            });
        }
        inventory.plots.push(Plot {
            plot_id,
            plot_size_acres: 0.2,
            slope_percent: Some(15.0),
            aspect_degrees: Some(180.0),
            elevation_ft: Some(2500.0),
            trees,
        });
    }

    assert_eq!(inventory.num_plots(), 50);
    assert_eq!(inventory.num_trees(), 1000);

    // All analysis should work on large inventories
    let metrics = compute_stand_metrics(&inventory);
    assert!(metrics.total_tpa > 0.0);

    let dist = DiameterDistribution::from_inventory(&inventory, 2.0);
    assert!(!dist.classes.is_empty());

    let stats = SamplingStatistics::compute(&inventory, 0.95).unwrap();
    assert!(stats.tpa.sampling_error_percent > 0.0);
    // With 50 plots, sampling error should be relatively small
    assert!(stats.tpa.sampling_error_percent < 50.0);

    let model = GrowthModel::Logistic {
        annual_rate: 0.03,
        carrying_capacity: 300.0,
    };
    let proj = project_growth(&inventory, &model, 10).unwrap();
    assert_eq!(proj.len(), 11);
}

// ============================================================================
// Input validation integration tests
// ============================================================================

/// Helper to write a CSV with one tree row and attempt to read it back.
fn write_and_read_csv(dbh: f64, height: &str, crown_ratio: &str, ef: f64, defect: &str) -> Result<ForestInventory, ForestError> {
    let dir = tempfile::tempdir().unwrap();
    let csv_path = dir.path().join("invalid.csv");
    let height_val = if height.is_empty() { "".to_string() } else { height.to_string() };
    let cr_val = if crown_ratio.is_empty() { "".to_string() } else { crown_ratio.to_string() };
    let defect_val = if defect.is_empty() { "".to_string() } else { defect.to_string() };
    let content = format!(
        "plot_id,tree_id,species_code,species_name,dbh,height,crown_ratio,status,expansion_factor,age,defect,plot_size_acres,slope_percent,aspect_degrees,elevation_ft\n\
         1,1,DF,Douglas Fir,{},{},{},Live,{},60,{},0.2,15,180,3000",
        dbh, height_val, cr_val, ef, defect_val
    );
    std::fs::write(&csv_path, content).unwrap();
    io::read_csv(&csv_path)
}

#[test]
fn test_csv_rejects_negative_dbh() {
    let result = write_and_read_csv(-5.0, "80", "0.5", 5.0, "");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("DBH must be positive"));
}

#[test]
fn test_csv_rejects_zero_dbh() {
    let result = write_and_read_csv(0.0, "80", "0.5", 5.0, "");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("DBH must be positive"));
}

#[test]
fn test_csv_rejects_negative_height() {
    let result = write_and_read_csv(12.0, "-10", "0.5", 5.0, "");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("height must be positive"));
}

#[test]
fn test_csv_rejects_crown_ratio_above_one() {
    let result = write_and_read_csv(12.0, "80", "1.5", 5.0, "");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("crown_ratio must be in 0.0..=1.0"));
}

#[test]
fn test_csv_rejects_negative_crown_ratio() {
    let result = write_and_read_csv(12.0, "80", "-0.1", 5.0, "");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("crown_ratio must be in 0.0..=1.0"));
}

#[test]
fn test_csv_rejects_zero_expansion_factor() {
    let result = write_and_read_csv(12.0, "80", "0.5", 0.0, "");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("expansion_factor must be positive"));
}

#[test]
fn test_csv_rejects_defect_above_one() {
    let result = write_and_read_csv(12.0, "80", "0.5", 5.0, "1.5");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("defect must be in 0.0..=1.0"));
}

#[test]
fn test_csv_accepts_valid_data() {
    let result = write_and_read_csv(12.0, "80", "0.5", 5.0, "0.1");
    assert!(result.is_ok());
}

#[test]
fn test_json_rejects_invalid_data() {
    // Create inventory with invalid tree, write to JSON, then read back
    let mut inventory = ForestInventory::new("Invalid");
    inventory.plots.push(Plot {
        plot_id: 1,
        plot_size_acres: 0.2,
        slope_percent: None,
        aspect_degrees: None,
        elevation_ft: None,
        trees: vec![Tree {
            tree_id: 1,
            plot_id: 1,
            species: Species {
                common_name: "Douglas Fir".to_string(),
                code: "DF".to_string(),
            },
            dbh: -5.0, // invalid
            height: Some(80.0),
            crown_ratio: Some(0.5),
            status: TreeStatus::Live,
            expansion_factor: 5.0,
            age: Some(60),
            defect: None,
        }],
    });

    let dir = tempfile::tempdir().unwrap();
    let json_path = dir.path().join("invalid.json");
    io::write_json(&inventory, &json_path, true).unwrap();

    let result = io::read_json(&json_path);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("DBH must be positive"));
}
