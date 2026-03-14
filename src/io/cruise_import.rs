use std::borrow::Cow;
use std::collections::HashMap;

use calamine::{DataType, Reader, Xlsx};

use crate::error::ForestError;
use crate::models::{ForestInventory, Plot, Species, Tree, TreeStatus, ValidationIssue};

use super::csv_io::EditableTreeRow;

/// Maximum plausible tree height in feet. Values above this are flagged as data entry errors.
const MAX_TREE_HEIGHT_FT: f64 = 300.0;

/// Derive a species code from a common name.
fn species_code(name: &str) -> String {
    match name.to_lowercase().as_str() {
        "ponderosa pine" => "PP".into(),
        "douglas fir" | "douglas-fir" => "DF".into(),
        "white fir" => "WF".into(),
        "red fir" => "RF".into(),
        "incense cedar" | "incense-cedar" => "IC".into(),
        "sugar pine" => "SP".into(),
        "jeffrey pine" => "JP".into(),
        "black oak" | "california black oak" => "BO".into(),
        "canyon live oak" => "CLO".into(),
        "tanoak" => "TO".into(),
        "pacific madrone" | "madrone" => "MA".into(),
        "giant sequoia" => "GS".into(),
        "lodgepole pine" => "LP".into(),
        "western white pine" => "WP".into(),
        "western red cedar" => "WRC".into(),
        "western hemlock" => "WH".into(),
        "sitka spruce" => "SS".into(),
        "bigleaf maple" => "BM".into(),
        "red alder" => "RA".into(),
        "null" | "" => "UNK".into(),
        other => other
            .split_whitespace()
            .filter_map(|w| w.chars().next())
            .collect::<String>()
            .to_uppercase(),
    }
}

/// Check if an Excel workbook has cruise-format sheets.
pub fn is_cruise_format(sheet_names: &[String]) -> bool {
    sheet_names.iter().any(|s| s.starts_with("Plot_form"))
}

/// Find a column index by case-insensitive prefix match.
fn find_col(headers: &[String], prefix: &str) -> Option<usize> {
    let lower = prefix.to_lowercase();
    headers
        .iter()
        .position(|h| h.to_lowercase().starts_with(&lower))
}

/// Find all column indices whose header contains `needle` (case-insensitive).
fn find_cols_containing(headers: &[String], needle: &str) -> Vec<usize> {
    let lower = needle.to_lowercase();
    headers
        .iter()
        .enumerate()
        .filter(|(_, h)| h.to_lowercase().contains(&lower))
        .map(|(i, _)| i)
        .collect()
}

/// Parsed row from a cruise sheet before conversion to Tree.
struct CruiseRow {
    stand_id: u32,
    plot_id: u32,
    species_name: String,
    dbh: f64,
    height: f64,
    sampling_method: String,
    raw_ef: f64,
    total_defect_pct: f64,
}

/// Parse all cruise rows from the Plot_form* sheets.
fn parse_cruise_sheets<RS: std::io::Read + std::io::Seek>(
    workbook: &mut Xlsx<RS>,
    sheet_names: &[String],
) -> Result<Vec<CruiseRow>, ForestError> {
    let cruise_sheets: Vec<&str> = sheet_names
        .iter()
        .filter(|s| s.starts_with("Plot_form"))
        .map(|s| s.as_str())
        .collect();

    if cruise_sheets.is_empty() {
        return Err(ForestError::ParseError(
            "No Plot_form sheets found".into(),
        ));
    }

    let mut all_rows = Vec::new();

    for sheet_name in cruise_sheets {
        let range = workbook
            .worksheet_range(sheet_name)
            .map_err(|e| ForestError::Excel(e.to_string()))?;

        let mut rows = range.rows();

        let header_row = rows.next().ok_or_else(|| {
            ForestError::Excel(format!("Sheet '{sheet_name}' is empty"))
        })?;
        let headers: Vec<String> = header_row
            .iter()
            .map(|c| c.to_string().trim().to_string())
            .collect();

        let stand_col = find_col(&headers, "Stand").ok_or_else(|| {
            ForestError::ParseError(format!("'Stand #' column not found in {sheet_name}"))
        })?;
        let plot_col = find_col(&headers, "Plot").ok_or_else(|| {
            ForestError::ParseError(format!("'Plot #' column not found in {sheet_name}"))
        })?;
        let species_col = find_col(&headers, "Species").ok_or_else(|| {
            ForestError::ParseError(format!("'Species' column not found in {sheet_name}"))
        })?;
        let dbh_col = find_col(&headers, "Diameter").ok_or_else(|| {
            ForestError::ParseError(format!(
                "'Diameter at Breast Height' column not found in {sheet_name}"
            ))
        })?;
        let height_col = find_col(&headers, "Total Height").ok_or_else(|| {
            ForestError::ParseError(format!(
                "'Total Height' column not found in {sheet_name}"
            ))
        })?;
        let method_col = find_col(&headers, "Sampling");
        let ef_col = find_col(&headers, "Expansion");
        let defect_cols = find_cols_containing(&headers, "defect");

        for row in rows {
            let get_f64 = |idx: usize| -> f64 {
                row.get(idx).and_then(|c| c.get_float()).unwrap_or(0.0)
            };
            let get_string = |idx: usize| -> String {
                row.get(idx)
                    .map(|c| c.to_string().trim().to_string())
                    .unwrap_or_default()
            };

            let total_defect_pct: f64 = defect_cols
                .iter()
                .filter_map(|&col| row.get(col).and_then(|c| c.get_float()))
                .sum();

            all_rows.push(CruiseRow {
                stand_id: get_f64(stand_col) as u32,
                plot_id: get_f64(plot_col) as u32,
                species_name: get_string(species_col),
                dbh: get_f64(dbh_col),
                height: get_f64(height_col),
                sampling_method: method_col
                    .map(|c| get_string(c))
                    .unwrap_or_default(),
                raw_ef: ef_col.map(|c| get_f64(c)).unwrap_or(0.0),
                total_defect_pct,
            });
        }
    }

    Ok(all_rows)
}

/// Compute per-tree expansion factor from cruise data.
///
/// - Variable radius plots (BAF): TPA = BAF / tree_basal_area
/// - Fixed plots: use expansion factor directly
fn compute_expansion_factor(method: &str, raw_ef: f64, dbh: f64) -> f64 {
    if method.to_lowercase().starts_with("var") && raw_ef > 0.0 {
        let ba = std::f64::consts::PI * (dbh / 2.0).powi(2) / 144.0;
        if ba > 0.0 {
            raw_ef / ba
        } else {
            0.0
        }
    } else {
        raw_ef
    }
}

/// Read a cruise-format Excel workbook into a ForestInventory.
pub fn read_cruise_excel<RS: std::io::Read + std::io::Seek>(
    workbook: &mut Xlsx<RS>,
    name: &str,
) -> Result<ForestInventory, ForestError> {
    let sheet_names: Vec<String> = workbook.sheet_names().to_vec();
    let cruise_rows = parse_cruise_sheets(workbook, &sheet_names)?;

    let mut plots: HashMap<(u32, u32), Plot> = HashMap::new();
    let mut tree_counters: HashMap<(u32, u32), u32> = HashMap::new();

    for cr in &cruise_rows {
        let key = (cr.stand_id, cr.plot_id);
        let composite_id = cr.stand_id * 1000 + cr.plot_id;

        plots.entry(key).or_insert_with(|| Plot {
            plot_id: composite_id,
            plot_size_acres: 0.2,
            slope_percent: None,
            aspect_degrees: None,
            elevation_ft: None,
            trees: Vec::new(),
            stand_id: Some(cr.stand_id),
        });

        // Null/zero DBH rows represent empty-plot markers — keep the plot but skip the tree
        let is_null = cr.dbh <= 0.0
            || cr.species_name.to_lowercase() == "null"
            || cr.species_name.is_empty();
        if is_null {
            continue;
        }

        let ef = compute_expansion_factor(&cr.sampling_method, cr.raw_ef, cr.dbh);
        let defect = if cr.total_defect_pct > 0.0 {
            Some((cr.total_defect_pct / 100.0).min(1.0))
        } else {
            None
        };

        let counter = tree_counters.entry(key).or_insert(0);
        *counter += 1;

        // Discard implausible heights (data entry errors)
        let height = if cr.height > 0.0 && cr.height <= MAX_TREE_HEIGHT_FT {
            Some(cr.height)
        } else {
            None
        };

        let tree = Tree {
            tree_id: *counter,
            plot_id: composite_id,
            species: Species {
                code: species_code(&cr.species_name),
                common_name: cr.species_name.clone(),
            },
            dbh: cr.dbh,
            height,
            crown_ratio: None,
            status: TreeStatus::Live,
            expansion_factor: ef,
            age: None,
            defect,
        };

        plots.get_mut(&key).unwrap().trees.push(tree);
    }

    let mut inventory = ForestInventory::new(name);
    let mut plot_list: Vec<Plot> = plots.into_values().collect();
    plot_list.sort_by_key(|p| p.plot_id);
    inventory.plots = plot_list;

    Ok(inventory)
}

/// Parse cruise-format Excel leniently, returning editable rows and validation issues.
pub fn parse_cruise_lenient<RS: std::io::Read + std::io::Seek>(
    workbook: &mut Xlsx<RS>,
    name: &str,
) -> Result<(String, Vec<EditableTreeRow>, Vec<ValidationIssue>), ForestError> {
    let sheet_names: Vec<String> = workbook.sheet_names().to_vec();
    let cruise_rows = parse_cruise_sheets(workbook, &sheet_names)?;

    let mut editable_rows = Vec::new();
    let mut issues = Vec::new();
    let mut tree_counters: HashMap<(u32, u32), u32> = HashMap::new();
    let mut row_index: usize = 0;

    for cr in &cruise_rows {
        let key = (cr.stand_id, cr.plot_id);
        let composite_id = cr.stand_id * 1000 + cr.plot_id;

        let is_null = cr.dbh <= 0.0
            || cr.species_name.to_lowercase() == "null"
            || cr.species_name.is_empty();
        if is_null {
            // Don't add a row, but track the plot via the row_index
            row_index += 1;
            continue;
        }

        let ef = compute_expansion_factor(&cr.sampling_method, cr.raw_ef, cr.dbh);
        let defect = if cr.total_defect_pct > 0.0 {
            Some((cr.total_defect_pct / 100.0).min(1.0))
        } else {
            None
        };

        let counter = tree_counters.entry(key).or_insert(0);
        *counter += 1;

        let code = species_code(&cr.species_name);

        // Flag implausible heights as validation issues
        let height = if cr.height > MAX_TREE_HEIGHT_FT {
            issues.push(ValidationIssue {
                plot_id: composite_id,
                tree_id: *counter,
                row_index,
                field: Cow::Borrowed("height"),
                message: Cow::Owned(format!(
                    "Height {:.0} ft exceeds {:.0} ft maximum — likely data entry error, set to empty",
                    cr.height, MAX_TREE_HEIGHT_FT
                )),
            });
            None
        } else if cr.height > 0.0 {
            Some(cr.height)
        } else {
            None
        };

        // Build a Tree to validate
        let tree = Tree {
            tree_id: *counter,
            plot_id: composite_id,
            species: Species {
                code: code.clone(),
                common_name: cr.species_name.clone(),
            },
            dbh: cr.dbh,
            height,
            crown_ratio: None,
            status: TreeStatus::Live,
            expansion_factor: ef,
            age: None,
            defect,
        };

        issues.extend(tree.validate_all(row_index));

        editable_rows.push(EditableTreeRow {
            row_index,
            plot_id: composite_id,
            tree_id: *counter,
            species_code: code,
            species_name: cr.species_name.clone(),
            dbh: cr.dbh,
            height,
            crown_ratio: None,
            status: "Live".to_string(),
            expansion_factor: ef,
            age: None,
            defect,
            plot_size_acres: Some(0.2),
            slope_percent: None,
            aspect_degrees: None,
            elevation_ft: None,
        });

        row_index += 1;
    }

    Ok((name.to_string(), editable_rows, issues))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_species_code_known() {
        assert_eq!(species_code("Ponderosa Pine"), "PP");
        assert_eq!(species_code("Douglas Fir"), "DF");
        assert_eq!(species_code("White Fir"), "WF");
        assert_eq!(species_code("Incense Cedar"), "IC");
        assert_eq!(species_code("Sugar Pine"), "SP");
    }

    #[test]
    fn test_species_code_unknown_uses_initials() {
        assert_eq!(species_code("Giant Chinquapin"), "GC");
        assert_eq!(species_code("Port Orford Cedar"), "POC");
    }

    #[test]
    fn test_species_code_null() {
        assert_eq!(species_code("null"), "UNK");
        assert_eq!(species_code(""), "UNK");
    }

    #[test]
    fn test_compute_ef_variable_plot() {
        // BAF 40, DBH 16": BA = pi * 64 / 144 = 1.396
        // TPA = 40 / 1.396 = 28.65
        let ef = compute_expansion_factor("var", 40.0, 16.0);
        assert!((ef - 28.65).abs() < 0.1);
    }

    #[test]
    fn test_compute_ef_fixed_plot() {
        let ef = compute_expansion_factor("fix", 50.0, 16.0);
        assert!((ef - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_compute_ef_variable_zero_dbh() {
        let ef = compute_expansion_factor("var", 40.0, 0.0);
        assert_eq!(ef, 0.0);
    }

    #[test]
    fn test_is_cruise_format() {
        assert!(is_cruise_format(&[
            "Sheet1".into(),
            "Plot_form".into(),
            "Plot_form2".into(),
        ]));
        assert!(!is_cruise_format(&["Sheet1".into(), "Data".into()]));
    }
}
