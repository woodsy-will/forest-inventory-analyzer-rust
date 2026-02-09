use serde::{Deserialize, Serialize};

use crate::error::ForestError;
use crate::models::ForestInventory;

/// Growth model type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GrowthModel {
    /// Simple exponential growth: V(t) = V0 * e^(r*t)
    Exponential {
        annual_rate: f64,
        /// Annual mortality rate as a proportion (e.g. 0.005 = 0.5%)
        mortality_rate: f64,
    },
    /// Logistic growth with carrying capacity: V(t) = K / (1 + ((K - V0)/V0) * e^(-r*t))
    Logistic {
        annual_rate: f64,
        carrying_capacity: f64,
        /// Annual mortality rate as a proportion (e.g. 0.005 = 0.5%)
        mortality_rate: f64,
    },
    /// Linear growth: V(t) = V0 + r*t
    Linear {
        annual_increment: f64,
        /// Annual TPA mortality (absolute, e.g. 0.5 TPA/year)
        mortality_rate: f64,
    },
}

/// A single year's growth projection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowthProjection {
    pub year: u32,
    pub tpa: f64,
    pub basal_area: f64,
    pub volume_cuft: f64,
    pub volume_bdft: f64,
}

/// Project stand growth over a number of years.
pub fn project_growth(
    inventory: &ForestInventory,
    model: &GrowthModel,
    years: u32,
) -> Result<Vec<GrowthProjection>, ForestError> {
    if inventory.num_plots() == 0 {
        return Err(ForestError::InsufficientData(
            "No plots available for growth projection".to_string(),
        ));
    }

    let initial_tpa = inventory.mean_tpa();
    let initial_ba = inventory.mean_basal_area();
    let initial_vol_cuft = inventory.mean_volume_cuft();
    let initial_vol_bdft = inventory.mean_volume_bdft();

    let mut projections = Vec::with_capacity(years as usize + 1);

    // Year 0 = current conditions
    projections.push(GrowthProjection {
        year: 0,
        tpa: initial_tpa,
        basal_area: initial_ba,
        volume_cuft: initial_vol_cuft,
        volume_bdft: initial_vol_bdft,
    });

    for year in 1..=years {
        let t = year as f64;

        let (tpa, ba, vol_cuft, vol_bdft) = match model {
            GrowthModel::Exponential {
                annual_rate,
                mortality_rate,
            } => {
                let factor = (annual_rate * t).exp();
                let tpa_factor = (-mortality_rate * t).exp();
                (
                    initial_tpa * tpa_factor,
                    initial_ba * factor,
                    initial_vol_cuft * factor,
                    initial_vol_bdft * factor,
                )
            }
            GrowthModel::Logistic {
                annual_rate,
                carrying_capacity,
                mortality_rate,
            } => {
                let apply_logistic = |v0: f64, k: f64| -> f64 {
                    if v0 <= 0.0 {
                        return 0.0;
                    }
                    k / (1.0 + ((k - v0) / v0) * (-annual_rate * t).exp())
                };
                // Scale carrying capacities relative to basal area capacity
                let ba_ratio = if initial_ba > 0.0 {
                    *carrying_capacity / initial_ba
                } else {
                    1.0
                };
                (
                    initial_tpa * (-mortality_rate * t).exp(),
                    apply_logistic(initial_ba, *carrying_capacity),
                    apply_logistic(initial_vol_cuft, initial_vol_cuft * ba_ratio),
                    apply_logistic(initial_vol_bdft, initial_vol_bdft * ba_ratio),
                )
            }
            GrowthModel::Linear {
                annual_increment,
                mortality_rate,
            } => (
                (initial_tpa - mortality_rate * t).max(0.0),
                initial_ba + annual_increment * t,
                initial_vol_cuft + annual_increment * t * 10.0, // rough volume scaling
                initial_vol_bdft + annual_increment * t * 50.0,
            ),
        };

        projections.push(GrowthProjection {
            year,
            tpa: tpa.max(0.0),
            basal_area: ba.max(0.0),
            volume_cuft: vol_cuft.max(0.0),
            volume_bdft: vol_bdft.max(0.0),
        });
    }

    Ok(projections)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Plot, Species, Tree, TreeStatus};

    fn make_tree(plot_id: u32, dbh: f64) -> Tree {
        Tree {
            tree_id: 1,
            plot_id,
            species: Species {
                common_name: "Douglas Fir".to_string(),
                code: "DF".to_string(),
            },
            dbh,
            height: Some(100.0),
            crown_ratio: Some(0.5),
            status: TreeStatus::Live,
            expansion_factor: 5.0,
            age: None,
            defect: None,
        }
    }

    fn make_plot(plot_id: u32, trees: Vec<Tree>) -> Plot {
        Plot {
            plot_id,
            plot_size_acres: 0.2,
            slope_percent: None,
            aspect_degrees: None,
            elevation_ft: None,
            trees,
        }
    }

    fn sample_inventory() -> ForestInventory {
        let mut inv = ForestInventory::new("Growth Test");
        inv.plots
            .push(make_plot(1, vec![make_tree(1, 14.0), make_tree(1, 16.0)]));
        inv.plots
            .push(make_plot(2, vec![make_tree(2, 12.0), make_tree(2, 18.0)]));
        inv
    }

    #[test]
    fn test_empty_inventory_error() {
        let inv = ForestInventory::new("Empty");
        let model = GrowthModel::Exponential {
            annual_rate: 0.03,
            mortality_rate: 0.005,
        };
        assert!(project_growth(&inv, &model, 10).is_err());
    }

    #[test]
    fn test_year_zero_matches_current() {
        let inv = sample_inventory();
        let model = GrowthModel::Exponential {
            annual_rate: 0.03,
            mortality_rate: 0.005,
        };
        let proj = project_growth(&inv, &model, 5).unwrap();
        assert_eq!(proj[0].year, 0);
        assert!((proj[0].tpa - inv.mean_tpa()).abs() < 0.001);
        assert!((proj[0].basal_area - inv.mean_basal_area()).abs() < 0.001);
    }

    #[test]
    fn test_projection_length() {
        let inv = sample_inventory();
        let model = GrowthModel::Exponential {
            annual_rate: 0.03,
            mortality_rate: 0.005,
        };
        let proj = project_growth(&inv, &model, 20).unwrap();
        assert_eq!(proj.len(), 21);
        assert_eq!(proj.first().unwrap().year, 0);
        assert_eq!(proj.last().unwrap().year, 20);
    }

    #[test]
    fn test_zero_years() {
        let inv = sample_inventory();
        let model = GrowthModel::Exponential {
            annual_rate: 0.03,
            mortality_rate: 0.005,
        };
        let proj = project_growth(&inv, &model, 0).unwrap();
        assert_eq!(proj.len(), 1);
        assert_eq!(proj[0].year, 0);
    }

    #[test]
    fn test_exponential_growth_increases_volume() {
        let inv = sample_inventory();
        let model = GrowthModel::Exponential {
            annual_rate: 0.03,
            mortality_rate: 0.005,
        };
        let proj = project_growth(&inv, &model, 10).unwrap();
        assert!(proj[10].basal_area > proj[0].basal_area);
        assert!(proj[10].volume_cuft > proj[0].volume_cuft);
        assert!(proj[10].volume_bdft > proj[0].volume_bdft);
    }

    #[test]
    fn test_exponential_tpa_decreases_mortality() {
        let inv = sample_inventory();
        let model = GrowthModel::Exponential {
            annual_rate: 0.03,
            mortality_rate: 0.005,
        };
        let proj = project_growth(&inv, &model, 10).unwrap();
        assert!(proj[10].tpa < proj[0].tpa);
    }

    #[test]
    fn test_exponential_monotonic_volume_increase() {
        let inv = sample_inventory();
        let model = GrowthModel::Exponential {
            annual_rate: 0.03,
            mortality_rate: 0.005,
        };
        let proj = project_growth(&inv, &model, 20).unwrap();
        for i in 1..proj.len() {
            assert!(proj[i].basal_area >= proj[i - 1].basal_area);
        }
    }

    #[test]
    fn test_logistic_growth_bounded() {
        let inv = sample_inventory();
        let model = GrowthModel::Logistic {
            annual_rate: 0.03,
            carrying_capacity: 300.0,
            mortality_rate: 0.005,
        };
        let proj = project_growth(&inv, &model, 100).unwrap();
        assert!(proj.last().unwrap().basal_area <= 300.0 + 0.1);
    }

    #[test]
    fn test_logistic_growth_increases() {
        let inv = sample_inventory();
        let model = GrowthModel::Logistic {
            annual_rate: 0.03,
            carrying_capacity: 300.0,
            mortality_rate: 0.005,
        };
        let proj = project_growth(&inv, &model, 10).unwrap();
        assert!(proj[10].basal_area >= proj[0].basal_area);
    }

    #[test]
    fn test_logistic_tpa_decreases() {
        let inv = sample_inventory();
        let model = GrowthModel::Logistic {
            annual_rate: 0.03,
            carrying_capacity: 300.0,
            mortality_rate: 0.005,
        };
        let proj = project_growth(&inv, &model, 10).unwrap();
        assert!(proj[10].tpa < proj[0].tpa);
    }

    #[test]
    fn test_linear_growth() {
        let inv = sample_inventory();
        let model = GrowthModel::Linear {
            annual_increment: 2.0,
            mortality_rate: 0.5,
        };
        let proj = project_growth(&inv, &model, 10).unwrap();
        let expected_ba = proj[0].basal_area + 2.0 * 10.0;
        assert!((proj[10].basal_area - expected_ba).abs() < 0.01);
    }

    #[test]
    fn test_linear_tpa_decreases_to_floor() {
        let inv = sample_inventory();
        let model = GrowthModel::Linear {
            annual_increment: 1.0,
            mortality_rate: 0.5,
        };
        let proj = project_growth(&inv, &model, 200).unwrap();
        assert!(proj.last().unwrap().tpa >= 0.0);
    }

    #[test]
    fn test_linear_volume_increase() {
        let inv = sample_inventory();
        let model = GrowthModel::Linear {
            annual_increment: 2.0,
            mortality_rate: 0.5,
        };
        let proj = project_growth(&inv, &model, 5).unwrap();
        let expected_vol = proj[0].volume_cuft + 2.0 * 5.0 * 10.0;
        assert!((proj[5].volume_cuft - expected_vol).abs() < 0.01);
    }

    #[test]
    fn test_all_projections_non_negative() {
        let inv = sample_inventory();
        let models: Vec<GrowthModel> = vec![
            GrowthModel::Exponential {
                annual_rate: 0.03,
                mortality_rate: 0.005,
            },
            GrowthModel::Logistic {
                annual_rate: 0.03,
                carrying_capacity: 300.0,
                mortality_rate: 0.005,
            },
            GrowthModel::Linear {
                annual_increment: 1.0,
                mortality_rate: 0.5,
            },
        ];
        for model in &models {
            let proj = project_growth(&inv, model, 50).unwrap();
            for p in &proj {
                assert!(p.tpa >= 0.0, "TPA negative at year {}", p.year);
                assert!(p.basal_area >= 0.0, "BA negative at year {}", p.year);
                assert!(p.volume_cuft >= 0.0, "Vol cuft negative at year {}", p.year);
                assert!(p.volume_bdft >= 0.0, "Vol bdft negative at year {}", p.year);
            }
        }
    }

    #[test]
    fn test_growth_model_json_roundtrip() {
        let models = vec![
            GrowthModel::Exponential {
                annual_rate: 0.03,
                mortality_rate: 0.005,
            },
            GrowthModel::Logistic {
                annual_rate: 0.05,
                carrying_capacity: 250.0,
                mortality_rate: 0.005,
            },
            GrowthModel::Linear {
                annual_increment: 1.5,
                mortality_rate: 0.5,
            },
        ];
        for model in &models {
            let json = serde_json::to_string(model).unwrap();
            let _deserialized: GrowthModel = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn test_growth_projection_json_roundtrip() {
        let proj = GrowthProjection {
            year: 5,
            tpa: 100.0,
            basal_area: 150.0,
            volume_cuft: 2000.0,
            volume_bdft: 10000.0,
        };
        let json = serde_json::to_string(&proj).unwrap();
        let deserialized: GrowthProjection = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.year, 5);
        assert!((deserialized.tpa - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_zero_mortality_no_tpa_decrease() {
        let inv = sample_inventory();
        let model = GrowthModel::Exponential {
            annual_rate: 0.03,
            mortality_rate: 0.0,
        };
        let proj = project_growth(&inv, &model, 10).unwrap();
        assert!((proj[10].tpa - proj[0].tpa).abs() < 0.001);
    }

    #[test]
    fn test_higher_mortality_lower_tpa() {
        let inv = sample_inventory();
        let low = GrowthModel::Exponential {
            annual_rate: 0.03,
            mortality_rate: 0.005,
        };
        let high = GrowthModel::Exponential {
            annual_rate: 0.03,
            mortality_rate: 0.05,
        };
        let proj_low = project_growth(&inv, &low, 10).unwrap();
        let proj_high = project_growth(&inv, &high, 10).unwrap();
        assert!(proj_high[10].tpa < proj_low[10].tpa);
    }

    #[test]
    fn test_zero_mortality_linear_no_tpa_decrease() {
        let inv = sample_inventory();
        let model = GrowthModel::Linear {
            annual_increment: 2.0,
            mortality_rate: 0.0,
        };
        let proj = project_growth(&inv, &model, 10).unwrap();
        assert!((proj[10].tpa - proj[0].tpa).abs() < 0.001);
    }

    #[test]
    fn test_zero_mortality_logistic_no_tpa_decrease() {
        let inv = sample_inventory();
        let model = GrowthModel::Logistic {
            annual_rate: 0.03,
            carrying_capacity: 300.0,
            mortality_rate: 0.0,
        };
        let proj = project_growth(&inv, &model, 10).unwrap();
        assert!((proj[10].tpa - proj[0].tpa).abs() < 0.001);
    }
}
