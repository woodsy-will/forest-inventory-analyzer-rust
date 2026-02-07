# Forest Inventory Analyzer

[![CI](https://github.com/YOUR_USERNAME/forest_inventory_analyzer_rust/actions/workflows/ci.yml/badge.svg)](https://github.com/YOUR_USERNAME/forest_inventory_analyzer_rust/actions/workflows/ci.yml)

A comprehensive forest inventory analysis tool built in Rust. Supports CSV, JSON, and Excel formats with statistical analysis, growth projections, and text-based visualization.

## Features

- **Stand Metrics** - Trees per acre, basal area, volume (cubic & board feet), quadratic mean diameter
- **Species Composition** - Breakdown by species with percentage of TPA and basal area
- **Statistical Analysis** - Confidence intervals, sampling error, standard error using Student's t-distribution
- **Diameter Distribution** - Text-based histogram of diameter classes
- **Growth Projections** - Exponential, logistic, and linear growth models
- **Multi-Format I/O** - Read/write CSV, JSON, and Excel (.xlsx) files
- **Format Conversion** - Convert between any supported formats

## Installation

```bash
# Clone the repository
git clone https://github.com/YOUR_USERNAME/forest_inventory_analyzer_rust.git
cd forest_inventory_analyzer_rust

# Build
cargo build --release

# The binary will be at target/release/forest-analyzer
```

## Usage

### Analyze Inventory Data

```bash
# Full analysis with default settings
forest-analyzer analyze --input data/samples/sample_inventory.csv

# Custom confidence level and diameter class width
forest-analyzer analyze --input inventory.csv --confidence 0.90 --diameter-class-width 4.0
```

### Growth Projections

```bash
# Logistic growth model, 30-year projection
forest-analyzer growth --input inventory.csv --years 30 --model logistic --rate 0.03 --capacity 300

# Exponential growth
forest-analyzer growth --input inventory.csv --model exponential --rate 0.02

# Linear growth
forest-analyzer growth --input inventory.csv --model linear --rate 2.0
```

### Convert Between Formats

```bash
# CSV to JSON
forest-analyzer convert --input inventory.csv --output inventory.json --pretty

# CSV to Excel
forest-analyzer convert --input inventory.csv --output inventory.xlsx

# Excel to CSV
forest-analyzer convert --input inventory.xlsx --output inventory.csv
```

### Quick Summary

```bash
forest-analyzer summary --input inventory.csv
```

## CSV Format

The expected CSV format includes these columns:

| Column | Type | Required | Description |
|--------|------|----------|-------------|
| plot_id | integer | Yes | Plot identifier |
| tree_id | integer | Yes | Tree identifier within plot |
| species_code | string | Yes | Species code (e.g., "DF") |
| species_name | string | Yes | Common name (e.g., "Douglas Fir") |
| dbh | float | Yes | Diameter at breast height (inches) |
| height | float | No | Total height (feet) |
| crown_ratio | float | No | Crown ratio (0.0 - 1.0) |
| status | string | Yes | Live, Dead, Cut, or Missing |
| expansion_factor | float | Yes | Trees represented per sample tree |
| age | integer | No | Age at breast height |
| defect | float | No | Defect percentage (0.0 - 1.0) |
| plot_size_acres | float | No | Plot size in acres (default: 0.2) |
| slope_percent | float | No | Slope percentage |
| aspect_degrees | float | No | Aspect in degrees |
| elevation_ft | float | No | Elevation in feet |

## Library Usage

```rust
use forest_inventory_analyzer::{
    io,
    analysis::{compute_stand_metrics, SamplingStatistics, DiameterDistribution},
};

fn main() -> anyhow::Result<()> {
    // Load data
    let inventory = io::read_csv("inventory.csv")?;

    // Compute metrics
    let metrics = compute_stand_metrics(&inventory);
    println!("TPA: {:.1}", metrics.total_tpa);
    println!("Basal Area: {:.1} sq ft/ac", metrics.total_basal_area);

    // Statistical analysis
    let stats = SamplingStatistics::compute(&inventory, 0.95)?;
    println!("BA 95% CI: {:.1} - {:.1}", stats.basal_area.lower, stats.basal_area.upper);

    Ok(())
}
```

## Development

```bash
# Run tests
cargo test

# Run clippy
cargo clippy --all-features

# Format code
cargo fmt

# Build documentation
cargo doc --open
```

## License

MIT
