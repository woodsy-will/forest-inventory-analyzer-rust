//! Statistical analysis and growth modeling for forest inventory data.
//!
//! Key types: [`Analyzer`] (high-level analysis runner), [`StandMetrics`], [`SamplingStatistics`],
//! [`DiameterDistribution`], and [`GrowthModel`] / [`GrowthProjection`] for stand-level
//! growth projections.

mod analyzer;
mod diameter_distribution;
mod growth;
mod metrics;
mod statistics;

pub use analyzer::Analyzer;
pub use diameter_distribution::{DiameterClass, DiameterDistribution};
pub use growth::{project_growth, GrowthModel, GrowthProjection};
pub use metrics::{compute_stand_metrics, SpeciesComposition, StandMetrics};
pub use statistics::{ConfidenceInterval, SamplingStatistics};
