//! Growth projection example: project stand growth using different models.
//!
//! Run from the project root:
//!   cargo run --example growth_projection

use std::path::Path;

use forest_inventory_analyzer::analysis::{Analyzer, GrowthModel};
use forest_inventory_analyzer::io::{CsvFormat, InventoryReader};
use forest_inventory_analyzer::visualization::print_growth_table;

fn main() {
    let path = Path::new("data/samples/sample_inventory.csv");
    let inventory = CsvFormat.read(path).expect("Failed to read CSV file");

    let analyzer = Analyzer::new(&inventory);

    // Logistic growth (approaches carrying capacity)
    println!("\n=== Logistic Growth Model ===");
    let logistic = GrowthModel::Logistic {
        annual_rate: 0.03,
        carrying_capacity: 300.0,
        mortality_rate: 0.005,
    };
    match analyzer.project_growth(&logistic, 20) {
        Ok(projections) => print_growth_table(&projections),
        Err(e) => eprintln!("Logistic projection failed: {e}"),
    }

    // Exponential growth (unbounded)
    println!("\n=== Exponential Growth Model ===");
    let exponential = GrowthModel::Exponential {
        annual_rate: 0.03,
        mortality_rate: 0.005,
    };
    match analyzer.project_growth(&exponential, 20) {
        Ok(projections) => print_growth_table(&projections),
        Err(e) => eprintln!("Exponential projection failed: {e}"),
    }
}
