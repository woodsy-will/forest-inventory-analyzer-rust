//! Text-based output formatting for stand summaries, species tables, and diameter histograms.
//!
//! Each output has a `print_*` variant (writes to stdout) and a `format_*` variant
//! (returns a `String`), making it easy to use in both CLI and programmatic contexts.

mod charts;
mod tables;

pub use charts::{format_diameter_histogram, print_diameter_histogram};
pub use tables::{
    format_growth_table, format_species_table, format_stand_summary, format_statistics_table,
    print_growth_table, print_species_table, print_stand_summary, print_statistics_table,
};
