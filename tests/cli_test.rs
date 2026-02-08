use std::path::PathBuf;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

use forest_inventory_analyzer::{
    io::write_csv,
    models::{ForestInventory, Plot, Species, Tree, TreeStatus},
};

/// Create a test inventory and write it to a CSV file in the given directory.
fn create_test_csv(dir: &TempDir) -> PathBuf {
    let path = dir.path().join("test_inventory.csv");
    let inv = sample_inventory();
    write_csv(&inv, &path).unwrap();
    path
}

fn sample_inventory() -> ForestInventory {
    let df = Species {
        common_name: "Douglas Fir".to_string(),
        code: "DF".to_string(),
    };
    let wrc = Species {
        common_name: "Western Red Cedar".to_string(),
        code: "WRC".to_string(),
    };

    let mut inv = ForestInventory::new("CLI Test");
    inv.plots.push(Plot {
        plot_id: 1,
        plot_size_acres: 0.2,
        slope_percent: Some(15.0),
        aspect_degrees: Some(180.0),
        elevation_ft: Some(1200.0),
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
                species: wrc,
                dbh: 14.0,
                height: Some(90.0),
                crown_ratio: Some(0.5),
                status: TreeStatus::Live,
                expansion_factor: 5.0,
                age: None,
                defect: None,
            },
        ],
    });
    inv
}

fn cmd() -> Command {
    Command::cargo_bin("forest-analyzer").unwrap()
}

// --- Analyze subcommand ---

#[test]
fn test_analyze_success() {
    let dir = TempDir::new().unwrap();
    let csv_path = create_test_csv(&dir);

    cmd()
        .args(["analyze", "--input", csv_path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Trees per Acre"))
        .stdout(predicate::str::contains("Basal Area"));
}

#[test]
fn test_analyze_custom_confidence() {
    let dir = TempDir::new().unwrap();
    let csv_path = create_test_csv(&dir);

    cmd()
        .args([
            "analyze",
            "--input",
            csv_path.to_str().unwrap(),
            "--confidence",
            "0.90",
        ])
        .assert()
        .success();
}

#[test]
fn test_analyze_custom_diameter_width() {
    let dir = TempDir::new().unwrap();
    let csv_path = create_test_csv(&dir);

    cmd()
        .args([
            "analyze",
            "--input",
            csv_path.to_str().unwrap(),
            "--diameter-class-width",
            "4.0",
        ])
        .assert()
        .success();
}

// --- Growth subcommand ---

#[test]
fn test_growth_exponential() {
    let dir = TempDir::new().unwrap();
    let csv_path = create_test_csv(&dir);

    cmd()
        .args([
            "growth",
            "--input",
            csv_path.to_str().unwrap(),
            "--model",
            "exponential",
            "--years",
            "10",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Year"));
}

#[test]
fn test_growth_logistic() {
    let dir = TempDir::new().unwrap();
    let csv_path = create_test_csv(&dir);

    cmd()
        .args([
            "growth",
            "--input",
            csv_path.to_str().unwrap(),
            "--model",
            "logistic",
            "--years",
            "10",
        ])
        .assert()
        .success();
}

#[test]
fn test_growth_linear() {
    let dir = TempDir::new().unwrap();
    let csv_path = create_test_csv(&dir);

    cmd()
        .args([
            "growth",
            "--input",
            csv_path.to_str().unwrap(),
            "--model",
            "linear",
            "--years",
            "5",
        ])
        .assert()
        .success();
}

#[test]
fn test_growth_invalid_model() {
    let dir = TempDir::new().unwrap();
    let csv_path = create_test_csv(&dir);

    cmd()
        .args([
            "growth",
            "--input",
            csv_path.to_str().unwrap(),
            "--model",
            "quadratic",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown growth model"));
}

#[test]
fn test_growth_with_mortality() {
    let dir = TempDir::new().unwrap();
    let csv_path = create_test_csv(&dir);

    cmd()
        .args([
            "growth",
            "--input",
            csv_path.to_str().unwrap(),
            "--model",
            "exponential",
            "--mortality",
            "0.01",
        ])
        .assert()
        .success();
}

// --- Convert subcommand ---

#[test]
fn test_convert_csv_to_json() {
    let dir = TempDir::new().unwrap();
    let csv_path = create_test_csv(&dir);
    let json_path = dir.path().join("output.json");

    cmd()
        .args([
            "convert",
            "--input",
            csv_path.to_str().unwrap(),
            "--output",
            json_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Success"));

    assert!(json_path.exists());
}

#[test]
fn test_convert_csv_to_excel() {
    let dir = TempDir::new().unwrap();
    let csv_path = create_test_csv(&dir);
    let xlsx_path = dir.path().join("output.xlsx");

    cmd()
        .args([
            "convert",
            "--input",
            csv_path.to_str().unwrap(),
            "--output",
            xlsx_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(xlsx_path.exists());
}

// --- Summary subcommand ---

#[test]
fn test_summary_success() {
    let dir = TempDir::new().unwrap();
    let csv_path = create_test_csv(&dir);

    cmd()
        .args(["summary", "--input", csv_path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Quick Summary"))
        .stdout(predicate::str::contains("Plots"))
        .stdout(predicate::str::contains("Species"));
}

// --- Error cases ---

#[test]
fn test_missing_file() {
    cmd()
        .args(["analyze", "--input", "nonexistent.csv"])
        .assert()
        .failure();
}

#[test]
fn test_no_subcommand() {
    cmd().assert().failure();
}

#[test]
fn test_missing_input_flag() {
    cmd().args(["analyze"]).assert().failure();
}

// --- Help and version ---

#[test]
fn test_help_flag() {
    cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Forest Inventory Analyzer"));
}

#[test]
fn test_version_flag() {
    cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("forest-analyzer"));
}
