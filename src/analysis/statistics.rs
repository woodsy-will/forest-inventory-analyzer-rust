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
    use crate::models::{Plot, Species, Tree, TreeStatus};

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

    fn make_tree_with_ef(plot_id: u32, dbh: f64, ef: f64) -> Tree {
        Tree {
            tree_id: 1,
            plot_id,
            species: Species {
                common_name: "Douglas Fir".to_string(),
                code: "DF".to_string(),
            },
            dbh,
            height: Some(80.0),
            crown_ratio: Some(0.5),
            status: TreeStatus::Live,
            expansion_factor: ef,
            age: None,
            defect: None,
        }
    }

    fn sample_inventory(num_plots: u32) -> ForestInventory {
        let mut inv = ForestInventory::new("Stats Test");
        for i in 1..=num_plots {
            // Vary expansion factor per plot to create between-plot variability
            let ef = 4.0 + i as f64; // 5.0, 6.0, 7.0, ...
            inv.plots.push(make_plot(i, vec![
                make_tree_with_ef(i, 12.0 + i as f64, ef),
                make_tree_with_ef(i, 14.0 + i as f64, ef),
            ]));
        }
        inv
    }

    // --- compute_ci tests ---

    #[test]
    fn test_compute_ci_basic() {
        let values = vec![10.0, 12.0, 11.0, 13.0, 9.0];
        let ci = compute_ci(&values, 0.95).unwrap();
        assert!((ci.mean - 11.0).abs() < 0.001);
        assert!(ci.lower < ci.mean);
        assert!(ci.upper > ci.mean);
        assert_eq!(ci.sample_size, 5);
        assert!((ci.confidence_level - 0.95).abs() < 0.001);
    }

    #[test]
    fn test_compute_ci_symmetric() {
        let values = vec![10.0, 12.0, 11.0, 13.0, 9.0];
        let ci = compute_ci(&values, 0.95).unwrap();
        let lower_margin = ci.mean - ci.lower;
        let upper_margin = ci.upper - ci.mean;
        assert!((lower_margin - upper_margin).abs() < 0.0001);
    }

    #[test]
    fn test_compute_ci_two_observations() {
        let values = vec![10.0, 20.0];
        let ci = compute_ci(&values, 0.95).unwrap();
        assert!((ci.mean - 15.0).abs() < 0.001);
        assert_eq!(ci.sample_size, 2);
        // Wide CI with only 2 obs
        assert!(ci.upper - ci.lower > 10.0);
    }

    #[test]
    fn test_compute_ci_insufficient_data() {
        let values = vec![10.0];
        assert!(compute_ci(&values, 0.95).is_err());
    }

    #[test]
    fn test_compute_ci_empty() {
        let values: Vec<f64> = vec![];
        assert!(compute_ci(&values, 0.95).is_err());
    }

    #[test]
    fn test_compute_ci_identical_values() {
        let values = vec![10.0, 10.0, 10.0, 10.0];
        let ci = compute_ci(&values, 0.95).unwrap();
        assert!((ci.mean - 10.0).abs() < 0.001);
        assert!((ci.std_error).abs() < 0.001);
        assert!((ci.lower - 10.0).abs() < 0.001);
        assert!((ci.upper - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_compute_ci_higher_confidence_wider() {
        let values = vec![10.0, 12.0, 11.0, 13.0, 9.0];
        let ci_90 = compute_ci(&values, 0.90).unwrap();
        let ci_95 = compute_ci(&values, 0.95).unwrap();
        let ci_99 = compute_ci(&values, 0.99).unwrap();
        let width_90 = ci_90.upper - ci_90.lower;
        let width_95 = ci_95.upper - ci_95.lower;
        let width_99 = ci_99.upper - ci_99.lower;
        assert!(width_95 > width_90);
        assert!(width_99 > width_95);
    }

    #[test]
    fn test_compute_ci_more_data_narrower() {
        let small = vec![10.0, 12.0, 11.0];
        let large = vec![10.0, 12.0, 11.0, 10.5, 11.5, 10.8, 11.2, 11.0, 10.3, 11.7];
        let ci_small = compute_ci(&small, 0.95).unwrap();
        let ci_large = compute_ci(&large, 0.95).unwrap();
        let width_small = ci_small.upper - ci_small.lower;
        let width_large = ci_large.upper - ci_large.lower;
        assert!(width_large < width_small);
    }

    #[test]
    fn test_sampling_error_percent() {
        let values = vec![10.0, 12.0, 11.0, 13.0, 9.0];
        let ci = compute_ci(&values, 0.95).unwrap();
        // Sampling error % = (margin / mean) * 100
        let margin = ci.upper - ci.mean;
        let expected_pct = (margin / ci.mean) * 100.0;
        assert!((ci.sampling_error_percent - expected_pct).abs() < 0.01);
    }

    #[test]
    fn test_sampling_error_percent_zero_mean() {
        let values = vec![-5.0, 5.0, -5.0, 5.0];
        let ci = compute_ci(&values, 0.95).unwrap();
        assert_eq!(ci.sampling_error_percent, 0.0);
    }

    // --- SamplingStatistics tests ---

    #[test]
    fn test_sampling_statistics_compute() {
        let inv = sample_inventory(5);
        let stats = SamplingStatistics::compute(&inv, 0.95).unwrap();
        assert!(stats.tpa.mean > 0.0);
        assert!(stats.basal_area.mean > 0.0);
        assert!(stats.volume_cuft.mean > 0.0);
        assert!(stats.volume_bdft.mean > 0.0);
        assert_eq!(stats.tpa.sample_size, 5);
    }

    #[test]
    fn test_sampling_statistics_insufficient_plots() {
        let inv = sample_inventory(1);
        assert!(SamplingStatistics::compute(&inv, 0.95).is_err());
    }

    #[test]
    fn test_sampling_statistics_empty_inventory() {
        let inv = ForestInventory::new("Empty");
        assert!(SamplingStatistics::compute(&inv, 0.95).is_err());
    }

    #[test]
    fn test_sampling_statistics_two_plots() {
        let inv = sample_inventory(2);
        let stats = SamplingStatistics::compute(&inv, 0.95).unwrap();
        assert_eq!(stats.tpa.sample_size, 2);
        // With 2 plots, CI should be wide
        assert!(stats.tpa.upper - stats.tpa.lower > 0.0);
    }

    #[test]
    fn test_sampling_statistics_confidence_levels() {
        let inv = sample_inventory(5);
        let stats_90 = SamplingStatistics::compute(&inv, 0.90).unwrap();
        let stats_95 = SamplingStatistics::compute(&inv, 0.95).unwrap();
        // 95% CI should be wider than 90% CI
        let width_90 = stats_90.tpa.upper - stats_90.tpa.lower;
        let width_95 = stats_95.tpa.upper - stats_95.tpa.lower;
        assert!(width_95 > width_90);
    }

    #[test]
    fn test_sampling_statistics_json_roundtrip() {
        let inv = sample_inventory(3);
        let stats = SamplingStatistics::compute(&inv, 0.95).unwrap();
        let json = serde_json::to_string(&stats).unwrap();
        let deserialized: SamplingStatistics = serde_json::from_str(&json).unwrap();
        assert!((deserialized.tpa.mean - stats.tpa.mean).abs() < 0.001);
    }
}
