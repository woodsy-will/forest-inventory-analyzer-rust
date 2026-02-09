//! Basic analysis example: load CSV, compute metrics, and display results.
//!
//! Run from the project root:
//!   cargo run --example basic_analysis

use std::path::Path;

use forest_inventory_analyzer::analysis::Analyzer;
use forest_inventory_analyzer::analysis::DiameterDistribution;
use forest_inventory_analyzer::io::{CsvFormat, InventoryReader};
use forest_inventory_analyzer::visualization::{
    print_diameter_histogram, print_species_table, print_stand_summary, print_statistics_table,
};

fn main() {
    let path = Path::new("data/samples/sample_inventory.csv");
    let reader = CsvFormat;

    let inventory = reader.read(path).expect("Failed to read CSV file");
    println!(
        "Loaded '{}': {} plots, {} trees",
        inventory.name,
        inventory.num_plots(),
        inventory.num_trees()
    );

    let analyzer = Analyzer::new(&inventory);

    // Stand metrics
    let metrics = analyzer.stand_metrics();
    print_stand_summary(&metrics);
    print_species_table(&metrics);

    // Diameter distribution
    let dist = DiameterDistribution::from_inventory(&inventory, 2.0);
    print_diameter_histogram(&dist);

    // Sampling statistics
    match analyzer.sampling_statistics(0.95) {
        Ok(stats) => print_statistics_table(&stats),
        Err(e) => eprintln!("Could not compute sampling statistics: {e}"),
    }
}
