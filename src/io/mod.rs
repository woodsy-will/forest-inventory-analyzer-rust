mod csv_io;
mod json_io;
mod excel_io;

use std::path::Path;

use crate::error::ForestError;
use crate::models::ForestInventory;

pub use csv_io::{read_csv, read_csv_from_bytes, write_csv};
pub use json_io::{read_json, read_json_from_bytes, write_json};
pub use excel_io::{read_excel, read_excel_from_bytes, write_excel};

pub(crate) use csv_io::{parse_csv_lenient, rows_to_inventory, EditableTreeRow};
pub(crate) use json_io::parse_json_lenient;
pub(crate) use excel_io::parse_excel_lenient;

/// Trait for reading forest inventory data from a file.
pub trait InventoryReader {
    fn read(&self, path: &Path) -> Result<ForestInventory, ForestError>;
}

/// Trait for writing forest inventory data to a file.
pub trait InventoryWriter {
    fn write(&self, inventory: &ForestInventory, path: &Path) -> Result<(), ForestError>;
}

/// CSV format reader/writer.
pub struct CsvFormat;

impl InventoryReader for CsvFormat {
    fn read(&self, path: &Path) -> Result<ForestInventory, ForestError> {
        read_csv(path)
    }
}

impl InventoryWriter for CsvFormat {
    fn write(&self, inventory: &ForestInventory, path: &Path) -> Result<(), ForestError> {
        write_csv(inventory, path)
    }
}

/// JSON format reader/writer.
pub struct JsonFormat {
    pub pretty: bool,
}

impl Default for JsonFormat {
    fn default() -> Self {
        Self { pretty: false }
    }
}

impl InventoryReader for JsonFormat {
    fn read(&self, path: &Path) -> Result<ForestInventory, ForestError> {
        read_json(path)
    }
}

impl InventoryWriter for JsonFormat {
    fn write(&self, inventory: &ForestInventory, path: &Path) -> Result<(), ForestError> {
        write_json(inventory, path, self.pretty)
    }
}

/// Excel (.xlsx) format reader/writer.
pub struct ExcelFormat;

impl InventoryReader for ExcelFormat {
    fn read(&self, path: &Path) -> Result<ForestInventory, ForestError> {
        read_excel(path)
    }
}

impl InventoryWriter for ExcelFormat {
    fn write(&self, inventory: &ForestInventory, path: &Path) -> Result<(), ForestError> {
        write_excel(inventory, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Plot, Species, Tree, TreeStatus};

    fn sample_inventory() -> ForestInventory {
        let mut inv = ForestInventory::new("IO Trait Test");
        inv.plots.push(Plot {
            plot_id: 1,
            plot_size_acres: 0.2,
            slope_percent: None,
            aspect_degrees: None,
            elevation_ft: None,
            trees: vec![
                Tree {
                    tree_id: 1,
                    plot_id: 1,
                    species: Species {
                        common_name: "Douglas Fir".to_string(),
                        code: "DF".to_string(),
                    },
                    dbh: 14.0,
                    height: Some(90.0),
                    crown_ratio: Some(0.5),
                    status: TreeStatus::Live,
                    expansion_factor: 5.0,
                    age: None,
                    defect: None,
                },
                Tree {
                    tree_id: 2,
                    plot_id: 1,
                    species: Species {
                        common_name: "Western Red Cedar".to_string(),
                        code: "WRC".to_string(),
                    },
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
        inv
    }

    #[test]
    fn test_csv_trait_roundtrip() {
        let inv = sample_inventory();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.csv");

        let writer: &dyn InventoryWriter = &CsvFormat;
        writer.write(&inv, &path).unwrap();

        let reader: &dyn InventoryReader = &CsvFormat;
        let loaded = reader.read(&path).unwrap();

        assert_eq!(loaded.num_plots(), inv.num_plots());
        assert_eq!(loaded.num_trees(), inv.num_trees());
    }

    #[test]
    fn test_json_trait_roundtrip() {
        let inv = sample_inventory();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.json");

        let writer: &dyn InventoryWriter = &JsonFormat { pretty: true };
        writer.write(&inv, &path).unwrap();

        let reader: &dyn InventoryReader = &JsonFormat::default();
        let loaded = reader.read(&path).unwrap();

        assert_eq!(loaded.num_plots(), inv.num_plots());
        assert_eq!(loaded.num_trees(), inv.num_trees());
        assert_eq!(loaded.plots[0].trees[0].dbh, 14.0);
    }

    #[test]
    fn test_json_format_default() {
        let fmt = JsonFormat::default();
        assert!(!fmt.pretty);
    }
}
