use forest_inventory_analyzer::{
    analysis::{
        compute_stand_metrics, ConfidenceInterval, DiameterClass, DiameterDistribution,
        GrowthProjection, SamplingStatistics, StandMetrics,
    },
    models::{ForestInventory, Plot, Species, Tree, TreeStatus},
    visualization::{
        format_diameter_histogram, format_growth_table, format_species_table,
        format_stand_summary, format_statistics_table,
    },
};

/// Build a deterministic inventory with fixed values for stable snapshots.
fn deterministic_inventory() -> ForestInventory {
    let df = Species {
        common_name: "Douglas Fir".to_string(),
        code: "DF".to_string(),
    };
    let wrc = Species {
        common_name: "Western Red Cedar".to_string(),
        code: "WRC".to_string(),
    };

    let mut inv = ForestInventory::new("Snapshot Test Stand");
    inv.plots.push(Plot {
        plot_id: 1,
        plot_size_acres: 0.2,
        slope_percent: Some(15.0),
        aspect_degrees: Some(180.0),
        elevation_ft: Some(3000.0),
        stand_id: None,
        trees: vec![
            Tree {
                tree_id: 1,
                plot_id: 1,
                species: df.clone(),
                dbh: 16.0,
                height: Some(100.0),
                crown_ratio: Some(0.5),
                status: TreeStatus::Live,
                expansion_factor: 5.0,
                age: None,
                defect: None,
            },
            Tree {
                tree_id: 2,
                plot_id: 1,
                species: wrc.clone(),
                dbh: 12.0,
                height: Some(80.0),
                crown_ratio: Some(0.6),
                status: TreeStatus::Live,
                expansion_factor: 5.0,
                age: None,
                defect: None,
            },
        ],
    });
    inv.plots.push(Plot {
        plot_id: 2,
        plot_size_acres: 0.2,
        slope_percent: None,
        aspect_degrees: None,
        elevation_ft: None,
        trees: vec![
            Tree {
                tree_id: 3,
                plot_id: 2,
                species: df.clone(),
                dbh: 18.0,
                height: Some(110.0),
                crown_ratio: Some(0.4),
                status: TreeStatus::Live,
                expansion_factor: 5.0,
                age: None,
                defect: None,
            },
            Tree {
                tree_id: 4,
                plot_id: 2,
                species: wrc.clone(),
                dbh: 14.0,
                height: Some(90.0),
                crown_ratio: Some(0.5),
                status: TreeStatus::Live,
                expansion_factor: 5.0,
                age: None,
                defect: None,
            },
        ],
        stand_id: None,
    });
    inv
}

/// Build deterministic StandMetrics from the test inventory.
fn deterministic_metrics() -> StandMetrics {
    let inv = deterministic_inventory();
    compute_stand_metrics(&inv)
}

/// Build deterministic SamplingStatistics with fixed values.
fn deterministic_statistics() -> SamplingStatistics {
    SamplingStatistics {
        tpa: ConfidenceInterval {
            mean: 10.0,
            std_error: 0.50,
            lower: 3.6,
            upper: 16.4,
            confidence_level: 0.95,
            sample_size: 2,
            sampling_error_percent: 63.7,
        },
        basal_area: ConfidenceInterval {
            mean: 7.1,
            std_error: 0.75,
            lower: -2.4,
            upper: 16.6,
            confidence_level: 0.95,
            sample_size: 2,
            sampling_error_percent: 133.8,
        },
        volume_cuft: ConfidenceInterval {
            mean: 250.0,
            std_error: 25.00,
            lower: -67.6,
            upper: 567.6,
            confidence_level: 0.95,
            sample_size: 2,
            sampling_error_percent: 127.0,
        },
        volume_bdft: ConfidenceInterval {
            mean: 1200.0,
            std_error: 120.00,
            lower: -324.5,
            upper: 2724.5,
            confidence_level: 0.95,
            sample_size: 2,
            sampling_error_percent: 127.0,
        },
    }
}

/// Build deterministic growth projections.
fn deterministic_projections() -> Vec<GrowthProjection> {
    vec![
        GrowthProjection {
            year: 0,
            tpa: 10.0,
            basal_area: 7.1,
            volume_cuft: 250.0,
            volume_bdft: 1200.0,
        },
        GrowthProjection {
            year: 5,
            tpa: 9.8,
            basal_area: 8.2,
            volume_cuft: 289.5,
            volume_bdft: 1389.7,
        },
        GrowthProjection {
            year: 10,
            tpa: 9.5,
            basal_area: 9.5,
            volume_cuft: 335.2,
            volume_bdft: 1609.4,
        },
        GrowthProjection {
            year: 15,
            tpa: 9.3,
            basal_area: 11.0,
            volume_cuft: 387.8,
            volume_bdft: 1862.3,
        },
        GrowthProjection {
            year: 20,
            tpa: 9.0,
            basal_area: 12.7,
            volume_cuft: 448.7,
            volume_bdft: 2155.0,
        },
    ]
}

/// Build a deterministic diameter distribution.
fn deterministic_distribution() -> DiameterDistribution {
    DiameterDistribution {
        class_width: 2.0,
        classes: vec![
            DiameterClass {
                lower: 12.0,
                upper: 14.0,
                midpoint: 13.0,
                tpa: 5.0,
                basal_area: 3.9,
                tree_count: 2,
            },
            DiameterClass {
                lower: 14.0,
                upper: 16.0,
                midpoint: 15.0,
                tpa: 2.5,
                basal_area: 2.7,
                tree_count: 1,
            },
            DiameterClass {
                lower: 16.0,
                upper: 18.0,
                midpoint: 17.0,
                tpa: 2.5,
                basal_area: 3.5,
                tree_count: 1,
            },
            DiameterClass {
                lower: 18.0,
                upper: 20.0,
                midpoint: 19.0,
                tpa: 2.5,
                basal_area: 4.4,
                tree_count: 1,
            },
        ],
    }
}

// Strip ANSI color codes from output for stable, readable snapshots.
fn strip_ansi(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip until 'm' (end of ANSI escape sequence)
            while let Some(&next) = chars.peek() {
                chars.next();
                if next == 'm' {
                    break;
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

#[test]
fn snapshot_stand_summary() {
    let metrics = deterministic_metrics();
    let output = strip_ansi(&format_stand_summary(&metrics));
    insta::assert_snapshot!(output);
}

#[test]
fn snapshot_species_table() {
    let metrics = deterministic_metrics();
    let output = strip_ansi(&format_species_table(&metrics));
    insta::assert_snapshot!(output);
}

#[test]
fn snapshot_statistics_table() {
    let stats = deterministic_statistics();
    let output = strip_ansi(&format_statistics_table(&stats));
    insta::assert_snapshot!(output);
}

#[test]
fn snapshot_growth_table() {
    let projections = deterministic_projections();
    let output = strip_ansi(&format_growth_table(&projections));
    insta::assert_snapshot!(output);
}

#[test]
fn snapshot_diameter_histogram() {
    let dist = deterministic_distribution();
    let output = strip_ansi(&format_diameter_histogram(&dist));
    insta::assert_snapshot!(output);
}
