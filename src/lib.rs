//! Forest inventory analysis library for timber cruising and stand-level metrics.
//!
//! Provides I/O in multiple formats (CSV, JSON, Excel, GeoJSON), statistical analysis,
//! diameter distributions, growth projections, text-based visualization, and an optional
//! web server (behind the `web` feature). Key entry points: [`Analyzer`], [`ForestInventory`],
//! [`InventoryReader`], and [`InventoryWriter`].

pub mod analysis;
pub mod config;
pub mod error;
pub mod io;
pub mod models;
pub mod visualization;

#[cfg(feature = "web")]
pub mod web;

pub use analysis::{
    Analyzer, ConfidenceInterval, DiameterClass, DiameterDistribution, GrowthModel,
    GrowthProjection, SamplingStatistics, SpeciesComposition, StandMetrics,
};
pub use config::AppConfig;
pub use error::ForestError;
pub use io::{GeoJsonFormat, InventoryReader, InventoryWriter};
pub use models::{
    ForestInventory, Plot, Species, Tree, TreeStatus, ValidationIssue, VolumeEquation,
};
