use std::path::Path;

use crate::error::ForestError;
use crate::models::ForestInventory;

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
