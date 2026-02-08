use std::path::Path;

use calamine::{open_workbook, DataType, Reader, Xlsx};
use rust_xlsxwriter::Workbook;

use crate::error::ForestError;
use crate::models::{ForestInventory, Plot, Species, Tree, TreeStatus};

/// Read forest inventory data from an Excel (.xlsx) file.
///
/// Expects a sheet with columns:
/// plot_id, tree_id, species_code, species_name, dbh, height, crown_ratio,
/// status, expansion_factor, age, defect, plot_size_acres, slope_percent,
/// aspect_degrees, elevation_ft
pub fn read_excel(path: impl AsRef<Path>) -> Result<ForestInventory, ForestError> {
    let path = path.as_ref();
    let mut workbook: Xlsx<_> = open_workbook(path)?;

    let sheet_name = workbook
        .sheet_names()
        .first()
        .cloned()
        .ok_or_else(|| ForestError::Excel("No sheets found in workbook".to_string()))?;

    let range = workbook
        .worksheet_range(&sheet_name)
        .map_err(|e| ForestError::Excel(e.to_string()))?;

    let mut plots: std::collections::HashMap<u32, Plot> = std::collections::HashMap::new();
    let mut rows = range.rows();

    // Skip header row
    rows.next();

    for row in rows {
        if row.len() < 9 {
            continue;
        }

        let get_f64 = |idx: usize| -> f64 {
            row.get(idx)
                .and_then(|c| c.get_float())
                .unwrap_or(0.0)
        };

        let get_opt_f64 = |idx: usize| -> Option<f64> {
            row.get(idx).and_then(|c| c.get_float())
        };

        let get_string = |idx: usize| -> String {
            row.get(idx)
                .map(|c| c.to_string())
                .unwrap_or_default()
        };

        let plot_id = get_f64(0) as u32;
        let tree_id = get_f64(1) as u32;
        let status_str = get_string(7);
        let status: TreeStatus = status_str.parse().unwrap_or_else(|_| {
            log::warn!(
                "Plot {plot_id}, Tree {tree_id}: unknown status '{status_str}', defaulting to Live"
            );
            TreeStatus::Live
        });

        let tree = Tree {
            tree_id,
            plot_id,
            species: Species {
                code: get_string(2),
                common_name: get_string(3),
            },
            dbh: get_f64(4),
            height: get_opt_f64(5),
            crown_ratio: get_opt_f64(6),
            status,
            expansion_factor: get_f64(8),
            age: get_opt_f64(9).map(|v| v as u32),
            defect: get_opt_f64(10),
        };

        tree.validate()?;

        let plot = plots.entry(plot_id).or_insert_with(|| Plot {
            plot_id,
            plot_size_acres: get_opt_f64(11).unwrap_or(0.2),
            slope_percent: get_opt_f64(12),
            aspect_degrees: get_opt_f64(13),
            elevation_ft: get_opt_f64(14),
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

/// Read forest inventory data from Excel bytes.
pub fn read_excel_from_bytes(data: &[u8], name: &str) -> Result<ForestInventory, ForestError> {
    use std::io::Write;
    let mut tmp = tempfile::NamedTempFile::new()?;
    tmp.write_all(data)?;
    tmp.flush()?;
    let mut inventory = read_excel(tmp.path())?;
    inventory.name = name.to_string();
    Ok(inventory)
}

/// Write forest inventory data to an Excel (.xlsx) file.
pub fn write_excel(
    inventory: &ForestInventory,
    path: impl AsRef<Path>,
) -> Result<(), ForestError> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    // Write headers
    let headers = [
        "plot_id",
        "tree_id",
        "species_code",
        "species_name",
        "dbh",
        "height",
        "crown_ratio",
        "status",
        "expansion_factor",
        "age",
        "defect",
        "plot_size_acres",
        "slope_percent",
        "aspect_degrees",
        "elevation_ft",
    ];

    for (col, header) in headers.iter().enumerate() {
        worksheet
            .write_string(0, col as u16, *header)
            .map_err(|e| ForestError::Excel(e.to_string()))?;
    }

    let mut row_idx: u32 = 1;
    for plot in &inventory.plots {
        for tree in &plot.trees {
            worksheet
                .write_number(row_idx, 0, tree.plot_id as f64)
                .map_err(|e| ForestError::Excel(e.to_string()))?;
            worksheet
                .write_number(row_idx, 1, tree.tree_id as f64)
                .map_err(|e| ForestError::Excel(e.to_string()))?;
            worksheet
                .write_string(row_idx, 2, &tree.species.code)
                .map_err(|e| ForestError::Excel(e.to_string()))?;
            worksheet
                .write_string(row_idx, 3, &tree.species.common_name)
                .map_err(|e| ForestError::Excel(e.to_string()))?;
            worksheet
                .write_number(row_idx, 4, tree.dbh)
                .map_err(|e| ForestError::Excel(e.to_string()))?;
            if let Some(h) = tree.height {
                worksheet
                    .write_number(row_idx, 5, h)
                    .map_err(|e| ForestError::Excel(e.to_string()))?;
            }
            if let Some(cr) = tree.crown_ratio {
                worksheet
                    .write_number(row_idx, 6, cr)
                    .map_err(|e| ForestError::Excel(e.to_string()))?;
            }
            worksheet
                .write_string(row_idx, 7, tree.status.to_string())
                .map_err(|e| ForestError::Excel(e.to_string()))?;
            worksheet
                .write_number(row_idx, 8, tree.expansion_factor)
                .map_err(|e| ForestError::Excel(e.to_string()))?;
            if let Some(age) = tree.age {
                worksheet
                    .write_number(row_idx, 9, age as f64)
                    .map_err(|e| ForestError::Excel(e.to_string()))?;
            }
            if let Some(defect) = tree.defect {
                worksheet
                    .write_number(row_idx, 10, defect)
                    .map_err(|e| ForestError::Excel(e.to_string()))?;
            }
            worksheet
                .write_number(row_idx, 11, plot.plot_size_acres)
                .map_err(|e| ForestError::Excel(e.to_string()))?;
            if let Some(slope) = plot.slope_percent {
                worksheet
                    .write_number(row_idx, 12, slope)
                    .map_err(|e| ForestError::Excel(e.to_string()))?;
            }
            if let Some(aspect) = plot.aspect_degrees {
                worksheet
                    .write_number(row_idx, 13, aspect)
                    .map_err(|e| ForestError::Excel(e.to_string()))?;
            }
            if let Some(elev) = plot.elevation_ft {
                worksheet
                    .write_number(row_idx, 14, elev)
                    .map_err(|e| ForestError::Excel(e.to_string()))?;
            }

            row_idx += 1;
        }
    }

    workbook
        .save(path.as_ref())
        .map_err(|e| ForestError::Excel(e.to_string()))?;

    Ok(())
}
