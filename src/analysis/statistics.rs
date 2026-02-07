use serde::{Deserialize, Serialize};
use statrs::distribution::{ContinuousCDF, StudentsT};

use crate::error::ForestError;
use crate::models::ForestInventory;

/// Confidence interval for a metric.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceInterval {
    pub mean: f64,
    pub std_error: f64,
    pub lower: f64,
    pub upper: f64,
    pub confidence_level: f64,
    pub sample_size: usize,
    pub sampling_error_percent: f64,
}

/// Complete sampling statistics for the inventory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingStatistics {
    pub tpa: ConfidenceInterval,
    pub basal_area: ConfidenceInterval,
    pub volume_cuft: ConfidenceInterval,
    pub volume_bdft: ConfidenceInterval,
}

impl SamplingStatistics {
    /// Compute sampling statistics from an inventory at a given confidence level (e.g. 0.95).
    pub fn compute(
        inventory: &ForestInventory,
        confidence: f64,
    ) -> Result<Self, ForestError> {
        let n = inventory.num_plots();
        if n < 2 {
            return Err(ForestError::InsufficientData(
                "Need at least 2 plots for statistical analysis".to_string(),
            ));
        }

        let tpa_values: Vec<f64> = inventory.plots.iter().map(|p| p.trees_per_acre()).collect();
        let ba_values: Vec<f64> = inventory
            .plots
            .iter()
            .map(|p| p.basal_area_per_acre())
            .collect();
        let vol_cuft_values: Vec<f64> = inventory
            .plots
            .iter()
            .map(|p| p.volume_cuft_per_acre())
            .collect();
        let vol_bdft_values: Vec<f64> = inventory
            .plots
            .iter()
            .map(|p| p.volume_bdft_per_acre())
            .collect();

        Ok(SamplingStatistics {
            tpa: compute_ci(&tpa_values, confidence)?,
            basal_area: compute_ci(&ba_values, confidence)?,
            volume_cuft: compute_ci(&vol_cuft_values, confidence)?,
            volume_bdft: compute_ci(&vol_bdft_values, confidence)?,
        })
    }
}

/// Compute a confidence interval from a set of values.
fn compute_ci(values: &[f64], confidence: f64) -> Result<ConfidenceInterval, ForestError> {
    let n = values.len();
    if n < 2 {
        return Err(ForestError::InsufficientData(
            "Need at least 2 observations".to_string(),
        ));
    }

    let mean = values.iter().sum::<f64>() / n as f64;
    let variance = values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1) as f64;
    let std_dev = variance.sqrt();
    let std_error = std_dev / (n as f64).sqrt();

    let df = (n - 1) as f64;
    let alpha = 1.0 - confidence;
    let t_dist =
        StudentsT::new(0.0, 1.0, df).map_err(|e| ForestError::AnalysisError(e.to_string()))?;
    let t_value = t_dist.inverse_cdf(1.0 - alpha / 2.0);

    let margin = t_value * std_error;
    let sampling_error_percent = if mean.abs() > f64::EPSILON {
        (margin / mean) * 100.0
    } else {
        0.0
    };

    Ok(ConfidenceInterval {
        mean,
        std_error,
        lower: mean - margin,
        upper: mean + margin,
        confidence_level: confidence,
        sample_size: n,
        sampling_error_percent,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_ci() {
        let values = vec![10.0, 12.0, 11.0, 13.0, 9.0];
        let ci = compute_ci(&values, 0.95).unwrap();
        assert!((ci.mean - 11.0).abs() < 0.001);
        assert!(ci.lower < ci.mean);
        assert!(ci.upper > ci.mean);
        assert_eq!(ci.sample_size, 5);
    }

    #[test]
    fn test_insufficient_data() {
        let values = vec![10.0];
        let result = compute_ci(&values, 0.95);
        assert!(result.is_err());
    }
}
