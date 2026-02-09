use std::path::Path;

use serde_json::{json, Value};

use crate::error::ForestError;
use crate::models::ForestInventory;

/// Write a forest inventory as a GeoJSON FeatureCollection.
///
/// Each plot becomes a Feature with a null geometry (no coordinates available)
/// and properties containing plot-level summary metrics plus tree details.
pub fn write_geojson(inventory: &ForestInventory, path: &Path, pretty: bool) -> Result<(), ForestError> {
    let features: Vec<Value> = inventory
        .plots
        .iter()
        .map(|plot| {
            let trees: Vec<Value> = plot
                .trees
                .iter()
                .map(|t| {
                    json!({
                        "tree_id": t.tree_id,
                        "species_code": t.species.code,
                        "species_name": t.species.common_name,
                        "dbh": t.dbh,
                        "height": t.height,
                        "crown_ratio": t.crown_ratio,
                        "status": format!("{:?}", t.status),
                        "expansion_factor": t.expansion_factor,
                        "age": t.age,
                        "defect": t.defect,
                    })
                })
                .collect();

            json!({
                "type": "Feature",
                "geometry": Value::Null,
                "properties": {
                    "plot_id": plot.plot_id,
                    "plot_size_acres": plot.plot_size_acres,
                    "slope_percent": plot.slope_percent,
                    "aspect_degrees": plot.aspect_degrees,
                    "elevation_ft": plot.elevation_ft,
                    "trees_per_acre": plot.trees_per_acre(),
                    "basal_area_per_acre": plot.basal_area_per_acre(),
                    "volume_cuft_per_acre": plot.volume_cuft_per_acre(),
                    "volume_bdft_per_acre": plot.volume_bdft_per_acre(),
                    "quadratic_mean_diameter": plot.quadratic_mean_diameter(),
                    "num_trees": plot.trees.len(),
                    "trees": trees,
                }
            })
        })
        .collect();

    let collection = json!({
        "type": "FeatureCollection",
        "features": features,
    });

    let content = if pretty {
        serde_json::to_string_pretty(&collection)?
    } else {
        serde_json::to_string(&collection)?
    };

    std::fs::write(path, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Plot, Species, Tree, TreeStatus};

    fn sample_inventory() -> ForestInventory {
        let mut inv = ForestInventory::new("GeoJSON Test");
        inv.plots.push(Plot {
            plot_id: 1,
            plot_size_acres: 0.2,
            slope_percent: Some(15.0),
            aspect_degrees: Some(180.0),
            elevation_ft: Some(3000.0),
            trees: vec![Tree {
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
            }],
        });
        inv
    }

    #[test]
    fn test_write_geojson_creates_valid_feature_collection() {
        let inv = sample_inventory();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.geojson");

        write_geojson(&inv, &path, true).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let parsed: Value = serde_json::from_str(&content).unwrap();

        assert_eq!(parsed["type"], "FeatureCollection");
        assert_eq!(parsed["features"].as_array().unwrap().len(), 1);

        let feature = &parsed["features"][0];
        assert_eq!(feature["type"], "Feature");
        assert!(feature["geometry"].is_null());
        assert_eq!(feature["properties"]["plot_id"], 1);
        assert_eq!(feature["properties"]["num_trees"], 1);
    }

    #[test]
    fn test_write_geojson_compact() {
        let inv = sample_inventory();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.geojson");

        write_geojson(&inv, &path, false).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        // Compact JSON has no newlines within the object
        assert!(!content.contains('\n'));
    }

    #[test]
    fn test_write_geojson_includes_tree_details() {
        let inv = sample_inventory();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.geojson");

        write_geojson(&inv, &path, true).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let parsed: Value = serde_json::from_str(&content).unwrap();

        let trees = &parsed["features"][0]["properties"]["trees"];
        assert_eq!(trees.as_array().unwrap().len(), 1);
        assert_eq!(trees[0]["species_code"], "DF");
        assert_eq!(trees[0]["dbh"], 14.0);
    }

    #[test]
    fn test_write_geojson_includes_plot_metrics() {
        let inv = sample_inventory();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.geojson");

        write_geojson(&inv, &path, true).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let parsed: Value = serde_json::from_str(&content).unwrap();

        let props = &parsed["features"][0]["properties"];
        assert!(props["trees_per_acre"].as_f64().unwrap() > 0.0);
        assert!(props["basal_area_per_acre"].as_f64().unwrap() > 0.0);
        assert!(props["quadratic_mean_diameter"].as_f64().unwrap() > 0.0);
    }

    #[test]
    fn test_write_geojson_empty_inventory() {
        let inv = ForestInventory::new("Empty");
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.geojson");

        write_geojson(&inv, &path, true).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let parsed: Value = serde_json::from_str(&content).unwrap();

        assert_eq!(parsed["type"], "FeatureCollection");
        assert!(parsed["features"].as_array().unwrap().is_empty());
    }
}
