use std::io::Read;
use std::path::Path;

use crate::error::ForestError;
use crate::models::{ForestInventory, Plot, Species, Tree, TreeStatus, ValidationIssue};

/// CSV row structure for tree data.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct TreeRow {
    plot_id: u32,
    tree_id: u32,
    species_code: String,
    species_name: String,
    dbh: f64,
    height: Option<f64>,
    crown_ratio: Option<f64>,
    status: String,
    expansion_factor: f64,
    age: Option<u32>,
    defect: Option<f64>,
    plot_size_acres: Option<f64>,
    slope_percent: Option<f64>,
    aspect_degrees: Option<f64>,
    elevation_ft: Option<f64>,
}

fn parse_csv_records<R: Read>(
    rdr: &mut csv::Reader<R>,
) -> Result<std::collections::HashMap<u32, Plot>, ForestError> {
    let mut plots: std::collections::HashMap<u32, Plot> = std::collections::HashMap::new();

    for result in rdr.deserialize() {
        let row: TreeRow = result?;
        let status: TreeStatus = row.status.parse()?;

        let tree = Tree {
            tree_id: row.tree_id,
            plot_id: row.plot_id,
            species: Species {
                common_name: row.species_name,
                code: row.species_code,
            },
            dbh: row.dbh,
            height: row.height,
            crown_ratio: row.crown_ratio,
            status,
            expansion_factor: row.expansion_factor,
            age: row.age,
            defect: row.defect,
        };

        tree.validate()?;

        let plot = plots.entry(row.plot_id).or_insert_with(|| Plot {
            plot_id: row.plot_id,
            plot_size_acres: row.plot_size_acres.unwrap_or(0.2),
            slope_percent: row.slope_percent,
            aspect_degrees: row.aspect_degrees,
            elevation_ft: row.elevation_ft,
            trees: Vec::new(),
        });

        plot.trees.push(tree);
    }

    Ok(plots)
}

/// Read forest inventory data from a CSV file.
pub fn read_csv(path: impl AsRef<Path>) -> Result<ForestInventory, ForestError> {
    let path = path.as_ref();
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .trim(csv::Trim::All)
        .from_path(path)?;

    let plots = parse_csv_records(&mut rdr)?;

    let mut inventory = ForestInventory::new(
        path.file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unknown".to_string()),
    );
    let mut plot_list: Vec<Plot> = plots.into_values().collect();
    plot_list.sort_by_key(|p| p.plot_id);
    inventory.plots = plot_list;

    Ok(inventory)
}

/// Read forest inventory data from CSV bytes.
pub fn read_csv_from_bytes(data: &[u8], name: &str) -> Result<ForestInventory, ForestError> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(data);

    let plots = parse_csv_records(&mut rdr)?;

    let mut inventory = ForestInventory::new(name);
    let mut plot_list: Vec<Plot> = plots.into_values().collect();
    plot_list.sort_by_key(|p| p.plot_id);
    inventory.plots = plot_list;

    Ok(inventory)
}

/// Write forest inventory summary data to a CSV file.
pub fn write_csv(inventory: &ForestInventory, path: impl AsRef<Path>) -> Result<(), ForestError> {
    let mut wtr = csv::Writer::from_path(path.as_ref())?;

    for plot in &inventory.plots {
        for tree in &plot.trees {
            let row = TreeRow {
                plot_id: tree.plot_id,
                tree_id: tree.tree_id,
                species_code: tree.species.code.clone(),
                species_name: tree.species.common_name.clone(),
                dbh: tree.dbh,
                height: tree.height,
                crown_ratio: tree.crown_ratio,
                status: tree.status.to_string(),
                expansion_factor: tree.expansion_factor,
                age: tree.age,
                defect: tree.defect,
                plot_size_acres: Some(plot.plot_size_acres),
                slope_percent: plot.slope_percent,
                aspect_degrees: plot.aspect_degrees,
                elevation_ft: plot.elevation_ft,
            };
            wtr.serialize(&row)?;
        }
    }

    wtr.flush()?;
    Ok(())
}

/// Flat, editable representation of a tree row for the web editor.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EditableTreeRow {
    pub row_index: usize,
    pub plot_id: u32,
    pub tree_id: u32,
    pub species_code: String,
    pub species_name: String,
    pub dbh: f64,
    pub height: Option<f64>,
    pub crown_ratio: Option<f64>,
    pub status: String,
    pub expansion_factor: f64,
    pub age: Option<u32>,
    pub defect: Option<f64>,
    pub plot_size_acres: Option<f64>,
    pub slope_percent: Option<f64>,
    pub aspect_degrees: Option<f64>,
    pub elevation_ft: Option<f64>,
}

/// Convert flat editable rows into a `ForestInventory`.
pub(crate) fn rows_to_inventory(name: &str, rows: &[EditableTreeRow]) -> ForestInventory {
    let mut plots: std::collections::HashMap<u32, Plot> = std::collections::HashMap::new();

    for row in rows {
        let status: TreeStatus = row.status.parse().unwrap_or(TreeStatus::Live);
        let tree = Tree {
            tree_id: row.tree_id,
            plot_id: row.plot_id,
            species: Species {
                code: row.species_code.clone(),
                common_name: row.species_name.clone(),
            },
            dbh: row.dbh,
            height: row.height,
            crown_ratio: row.crown_ratio,
            status,
            expansion_factor: row.expansion_factor,
            age: row.age,
            defect: row.defect,
        };

        let plot = plots.entry(row.plot_id).or_insert_with(|| Plot {
            plot_id: row.plot_id,
            plot_size_acres: row.plot_size_acres.unwrap_or(0.2),
            slope_percent: row.slope_percent,
            aspect_degrees: row.aspect_degrees,
            elevation_ft: row.elevation_ft,
            trees: Vec::new(),
        });

        plot.trees.push(tree);
    }

    let mut inventory = ForestInventory::new(name);
    let mut plot_list: Vec<Plot> = plots.into_values().collect();
    plot_list.sort_by_key(|p| p.plot_id);
    inventory.plots = plot_list;
    inventory
}

/// Parse CSV leniently: collect all validation issues instead of failing on the first.
///
/// CSV **format** errors (missing columns, type mismatches) are still fatal.
/// Returns all rows (including invalid ones) + all validation issues.
pub(crate) fn parse_csv_lenient(
    data: &[u8],
    name: &str,
) -> Result<(String, Vec<EditableTreeRow>, Vec<ValidationIssue>), ForestError> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(data);

    let mut rows = Vec::new();
    let mut issues = Vec::new();

    for (row_index, result) in rdr.deserialize().enumerate() {
        let csv_row: TreeRow = result?;

        // Try to parse status; default to "Live" on error and record issue
        let status_str = csv_row.status.clone();
        let status: TreeStatus = match status_str.parse() {
            Ok(s) => s,
            Err(_) => {
                issues.push(ValidationIssue {
                    plot_id: csv_row.plot_id,
                    tree_id: csv_row.tree_id,
                    row_index,
                    field: "status".to_string(),
                    message: format!("Unknown tree status '{}', defaulting to Live", status_str),
                });
                TreeStatus::Live
            }
        };

        let tree = Tree {
            tree_id: csv_row.tree_id,
            plot_id: csv_row.plot_id,
            species: Species {
                common_name: csv_row.species_name.clone(),
                code: csv_row.species_code.clone(),
            },
            dbh: csv_row.dbh,
            height: csv_row.height,
            crown_ratio: csv_row.crown_ratio,
            status: status.clone(),
            expansion_factor: csv_row.expansion_factor,
            age: csv_row.age,
            defect: csv_row.defect,
        };

        // Validate leniently
        issues.extend(tree.validate_all(row_index));

        rows.push(EditableTreeRow {
            row_index,
            plot_id: csv_row.plot_id,
            tree_id: csv_row.tree_id,
            species_code: csv_row.species_code,
            species_name: csv_row.species_name,
            dbh: csv_row.dbh,
            height: csv_row.height,
            crown_ratio: csv_row.crown_ratio,
            status: status.to_string(),
            expansion_factor: csv_row.expansion_factor,
            age: csv_row.age,
            defect: csv_row.defect,
            plot_size_acres: csv_row.plot_size_acres,
            slope_percent: csv_row.slope_percent,
            aspect_degrees: csv_row.aspect_degrees,
            elevation_ft: csv_row.elevation_ft,
        });
    }

    Ok((name.to_string(), rows, issues))
}
