use std::path::Path;

use crate::error::ForestError;
use crate::models::{ForestInventory, Plot, Species, Tree, TreeStatus};

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

/// Read forest inventory data from a CSV file.
pub fn read_csv(path: impl AsRef<Path>) -> Result<ForestInventory, ForestError> {
    let path = path.as_ref();
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .trim(csv::Trim::All)
        .from_path(path)?;

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
