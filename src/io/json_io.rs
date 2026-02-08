use std::path::Path;

use crate::error::ForestError;
use crate::models::{ForestInventory, ValidationIssue};

use super::csv_io::EditableTreeRow;

/// Read forest inventory data from a JSON file.
pub fn read_json(path: impl AsRef<Path>) -> Result<ForestInventory, ForestError> {
    let content = std::fs::read_to_string(path.as_ref())?;
    let inventory: ForestInventory = serde_json::from_str(&content)?;
    for plot in &inventory.plots {
        for tree in &plot.trees {
            tree.validate()?;
        }
    }
    Ok(inventory)
}

/// Read forest inventory data from JSON bytes.
pub fn read_json_from_bytes(data: &[u8], name: &str) -> Result<ForestInventory, ForestError> {
    let content = std::str::from_utf8(data)
        .map_err(|e| ForestError::ParseError(format!("Invalid UTF-8: {e}")))?;
    let mut inventory: ForestInventory = serde_json::from_str(content)?;
    for plot in &inventory.plots {
        for tree in &plot.trees {
            tree.validate()?;
        }
    }
    inventory.name = name.to_string();
    Ok(inventory)
}

/// Write forest inventory data to a JSON file.
pub fn write_json(
    inventory: &ForestInventory,
    path: impl AsRef<Path>,
    pretty: bool,
) -> Result<(), ForestError> {
    let content = if pretty {
        serde_json::to_string_pretty(inventory)?
    } else {
        serde_json::to_string(inventory)?
    };
    std::fs::write(path.as_ref(), content)?;
    Ok(())
}

/// Parse JSON leniently: deserialize the inventory, flatten to editable rows,
/// validate all trees, and collect issues.
pub(crate) fn parse_json_lenient(
    data: &[u8],
    name: &str,
) -> Result<(String, Vec<EditableTreeRow>, Vec<ValidationIssue>), ForestError> {
    let content = std::str::from_utf8(data)
        .map_err(|e| ForestError::ParseError(format!("Invalid UTF-8: {e}")))?;
    let inventory: ForestInventory = serde_json::from_str(content)?;

    let mut rows = Vec::new();
    let mut issues = Vec::new();
    let mut row_index: usize = 0;

    for plot in &inventory.plots {
        for tree in &plot.trees {
            issues.extend(tree.validate_all(row_index));

            rows.push(EditableTreeRow {
                row_index,
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
            });

            row_index += 1;
        }
    }

    Ok((name.to_string(), rows, issues))
}
