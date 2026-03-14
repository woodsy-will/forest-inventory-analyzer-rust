//! Core domain types for forest inventory data.
//!
//! Key types: [`ForestInventory`] (top-level container), [`Plot`], [`Tree`], [`Species`],
//! [`TreeStatus`], and [`VolumeEquation`].

mod inventory;
mod plot;
mod tree;
mod volume;

pub use inventory::ForestInventory;
pub use plot::Plot;
pub use tree::{Species, Tree, TreeStatus, ValidationIssue};
pub use volume::VolumeEquation;
