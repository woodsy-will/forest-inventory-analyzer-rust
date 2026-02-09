//! Format conversion example: convert between CSV, JSON, and Excel.
//!
//! Run from the project root:
//!   cargo run --example format_conversion

use std::path::Path;

use forest_inventory_analyzer::io::{
    CsvFormat, ExcelFormat, InventoryReader, InventoryWriter, JsonFormat,
};

fn main() {
    let input = Path::new("data/samples/sample_inventory.csv");
    let inventory = CsvFormat.read(input).expect("Failed to read CSV file");
    println!(
        "Loaded '{}': {} plots, {} trees",
        inventory.name,
        inventory.num_plots(),
        inventory.num_trees()
    );

    // Write JSON (pretty-printed)
    let json_path = Path::new("output_example.json");
    let json_writer = JsonFormat { pretty: true };
    json_writer
        .write(&inventory, json_path)
        .expect("Failed to write JSON");
    println!("Wrote {}", json_path.display());

    // Write Excel
    let xlsx_path = Path::new("output_example.xlsx");
    ExcelFormat
        .write(&inventory, xlsx_path)
        .expect("Failed to write Excel");
    println!("Wrote {}", xlsx_path.display());

    // Round-trip: read back from JSON and verify
    let reloaded = JsonFormat::default()
        .read(json_path)
        .expect("Failed to read back JSON");
    assert_eq!(reloaded.num_plots(), inventory.num_plots());
    assert_eq!(reloaded.num_trees(), inventory.num_trees());
    println!("Round-trip verified: JSON matches original");

    // Clean up temp output files
    let _ = std::fs::remove_file(json_path);
    let _ = std::fs::remove_file(xlsx_path);
    println!("Cleaned up output files");
}
