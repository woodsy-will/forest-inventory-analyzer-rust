use crate::analysis::{
    compute_stand_metrics, project_growth, DiameterDistribution, GrowthModel, GrowthProjection,
    SamplingStatistics, StandMetrics,
};
use crate::error::ForestError;
use crate::models::ForestInventory;

/// Unified analysis API that groups all analysis operations on an inventory.
pub struct Analyzer<'a> {
    inventory: &'a ForestInventory,
}

impl<'a> Analyzer<'a> {
    /// Create a new Analyzer for the given inventory.
    pub fn new(inventory: &'a ForestInventory) -> Self {
        Self { inventory }
    }

    /// Compute stand-level metrics (TPA, BA, volume, QMD, species composition).
    pub fn stand_metrics(&self) -> StandMetrics {
        compute_stand_metrics(self.inventory)
    }

    /// Compute sampling statistics at the given confidence level (e.g. 0.95).
    pub fn sampling_statistics(&self, confidence: f64) -> Result<SamplingStatistics, ForestError> {
        SamplingStatistics::compute(self.inventory, confidence)
    }

    /// Build a diameter distribution with the given class width in inches.
    pub fn diameter_distribution(&self, class_width: f64) -> DiameterDistribution {
        DiameterDistribution::from_inventory(self.inventory, class_width)
    }

    /// Project stand growth over the given number of years using the specified model.
    pub fn project_growth(
        &self,
        model: &GrowthModel,
        years: u32,
    ) -> Result<Vec<GrowthProjection>, ForestError> {
        project_growth(self.inventory, model, years)
    }
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
        let mut inv = ForestInventory::new("Analyzer Test");
        inv.plots
            .push(make_plot(1, vec![make_tree(1, 14.0), make_tree(1, 16.0)]));
        inv.plots
            .push(make_plot(2, vec![make_tree(2, 12.0), make_tree(2, 18.0)]));
        inv
    }

    #[test]
    fn test_stand_metrics_matches_standalone() {
        let inv = sample_inventory();
        let analyzer = Analyzer::new(&inv);
        let from_analyzer = analyzer.stand_metrics();
        let from_standalone = compute_stand_metrics(&inv);
        assert!((from_analyzer.total_tpa - from_standalone.total_tpa).abs() < 0.001);
        assert!(
            (from_analyzer.total_basal_area - from_standalone.total_basal_area).abs() < 0.001
        );
        assert!(
            (from_analyzer.quadratic_mean_diameter - from_standalone.quadratic_mean_diameter)
                .abs()
                < 0.001
        );
    }

    #[test]
    fn test_sampling_statistics_matches_standalone() {
        let inv = sample_inventory();
        let analyzer = Analyzer::new(&inv);
        let from_analyzer = analyzer.sampling_statistics(0.95).unwrap();
        let from_standalone = SamplingStatistics::compute(&inv, 0.95).unwrap();
        assert!((from_analyzer.tpa.mean - from_standalone.tpa.mean).abs() < 0.001);
        assert!(
            (from_analyzer.basal_area.mean - from_standalone.basal_area.mean).abs() < 0.001
        );
    }

    #[test]
    fn test_diameter_distribution_matches_standalone() {
        let inv = sample_inventory();
        let analyzer = Analyzer::new(&inv);
        let from_analyzer = analyzer.diameter_distribution(2.0);
        let from_standalone = DiameterDistribution::from_inventory(&inv, 2.0);
        assert_eq!(from_analyzer.classes.len(), from_standalone.classes.len());
        assert_eq!(from_analyzer.class_width, from_standalone.class_width);
    }

    #[test]
    fn test_project_growth_matches_standalone() {
        let inv = sample_inventory();
        let analyzer = Analyzer::new(&inv);
        let model = GrowthModel::Exponential {
            annual_rate: 0.03,
            mortality_rate: 0.005,
        };
        let from_analyzer = analyzer.project_growth(&model, 10).unwrap();
        let from_standalone = project_growth(&inv, &model, 10).unwrap();
        assert_eq!(from_analyzer.len(), from_standalone.len());
        assert!((from_analyzer[10].basal_area - from_standalone[10].basal_area).abs() < 0.001);
    }

    #[test]
    fn test_analyzer_empty_inventory() {
        let inv = ForestInventory::new("Empty");
        let analyzer = Analyzer::new(&inv);
        let metrics = analyzer.stand_metrics();
        assert_eq!(metrics.total_tpa, 0.0);
        assert!(analyzer.sampling_statistics(0.95).is_err());
    }
}
