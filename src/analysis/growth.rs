use serde::{Deserialize, Serialize};

use crate::error::ForestError;
use crate::models::ForestInventory;

/// Growth model type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GrowthModel {
    /// Simple exponential growth: V(t) = V0 * e^(r*t)
    Exponential { annual_rate: f64 },
    /// Logistic growth with carrying capacity: V(t) = K / (1 + ((K - V0)/V0) * e^(-r*t))
    Logistic {
        annual_rate: f64,
        carrying_capacity: f64,
    },
    /// Linear growth: V(t) = V0 + r*t
    Linear { annual_increment: f64 },
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
            GrowthModel::Exponential { annual_rate } => {
                let factor = (annual_rate * t).exp();
                // TPA typically decreases slightly due to mortality
                let mortality_rate = 0.005; // 0.5% annual mortality
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
                    initial_tpa * (-0.005 * t).exp(), // slight mortality
                    apply_logistic(initial_ba, *carrying_capacity),
                    apply_logistic(initial_vol_cuft, initial_vol_cuft * ba_ratio),
                    apply_logistic(initial_vol_bdft, initial_vol_bdft * ba_ratio),
                )
            }
            GrowthModel::Linear { annual_increment } => (
                (initial_tpa - 0.5 * t).max(0.0), // slight mortality
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
