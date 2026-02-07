use forest_inventory_analyzer::{
    analysis::{
        compute_stand_metrics, project_growth, DiameterDistribution, GrowthModel,
        SamplingStatistics,
    },
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
fn test_sampling_statistics() {
    let inventory = create_test_inventory();
    let stats = SamplingStatistics::compute(&inventory, 0.95).unwrap();

    assert!(stats.tpa.mean > 0.0);
    assert!(stats.tpa.lower < stats.tpa.upper);
    assert_eq!(stats.tpa.sample_size, 3);
    assert!((stats.tpa.confidence_level - 0.95).abs() < 0.001);
}

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
fn test_species_list() {
    let inventory = create_test_inventory();
    let species = inventory.species_list();

    assert_eq!(species.len(), 2);
    assert!(species.iter().any(|s| s.code == "DF"));
    assert!(species.iter().any(|s| s.code == "WRC"));
}

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
fn test_tree_status_parsing() {
    assert_eq!("live".parse::<TreeStatus>().unwrap(), TreeStatus::Live);
    assert_eq!("L".parse::<TreeStatus>().unwrap(), TreeStatus::Live);
    assert_eq!("dead".parse::<TreeStatus>().unwrap(), TreeStatus::Dead);
    assert_eq!("D".parse::<TreeStatus>().unwrap(), TreeStatus::Dead);
    assert_eq!("cut".parse::<TreeStatus>().unwrap(), TreeStatus::Cut);
    assert!("unknown".parse::<TreeStatus>().is_err());
}
