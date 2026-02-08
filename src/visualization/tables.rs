use colored::Colorize;
use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Cell, ContentArrangement, Table};

use crate::analysis::{GrowthProjection, SamplingStatistics, StandMetrics};

/// Format a stand summary table as a string.
pub fn format_stand_summary(metrics: &StandMetrics) -> String {
    let mut output = String::new();
    output.push_str(&format!("\n{}\n", "Stand Summary".bold().green()));
    output.push_str(&format!("{}\n", "=".repeat(50)));

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["Metric", "Value", "Unit"]);

    table.add_row(vec![
        Cell::new("Trees per Acre"),
        Cell::new(format!("{:.1}", metrics.total_tpa)),
        Cell::new("TPA"),
    ]);
    table.add_row(vec![
        Cell::new("Basal Area"),
        Cell::new(format!("{:.1}", metrics.total_basal_area)),
        Cell::new("sq ft/acre"),
    ]);
    table.add_row(vec![
        Cell::new("Volume (cubic ft)"),
        Cell::new(format!("{:.1}", metrics.total_volume_cuft)),
        Cell::new("cu ft/acre"),
    ]);
    table.add_row(vec![
        Cell::new("Volume (board ft)"),
        Cell::new(format!("{:.0}", metrics.total_volume_bdft)),
        Cell::new("bd ft/acre"),
    ]);
    table.add_row(vec![
        Cell::new("QMD"),
        Cell::new(format!("{:.1}", metrics.quadratic_mean_diameter)),
        Cell::new("inches"),
    ]);
    if let Some(h) = metrics.mean_height {
        table.add_row(vec![
            Cell::new("Mean Height"),
            Cell::new(format!("{:.1}", h)),
            Cell::new("feet"),
        ]);
    }
    table.add_row(vec![
        Cell::new("Number of Species"),
        Cell::new(format!("{}", metrics.num_species)),
        Cell::new(""),
    ]);

    output.push_str(&format!("{table}"));
    output
}

/// Print a formatted stand summary table.
pub fn print_stand_summary(metrics: &StandMetrics) {
    print!("{}", format_stand_summary(metrics));
}

/// Format species composition table as a string.
pub fn format_species_table(metrics: &StandMetrics) -> String {
    let mut output = String::new();
    output.push_str(&format!("\n{}\n", "Species Composition".bold().green()));
    output.push_str(&format!("{}\n", "=".repeat(50)));

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            "Species",
            "Code",
            "TPA",
            "% TPA",
            "BA/ac",
            "% BA",
            "Mean DBH",
        ]);

    for sp in &metrics.species_composition {
        table.add_row(vec![
            Cell::new(&sp.species.common_name),
            Cell::new(&sp.species.code),
            Cell::new(format!("{:.1}", sp.tpa)),
            Cell::new(format!("{:.1}%", sp.percent_tpa)),
            Cell::new(format!("{:.1}", sp.basal_area)),
            Cell::new(format!("{:.1}%", sp.percent_basal_area)),
            Cell::new(format!("{:.1}\"", sp.mean_dbh)),
        ]);
    }

    output.push_str(&format!("{table}"));
    output
}

/// Print species composition table.
pub fn print_species_table(metrics: &StandMetrics) {
    print!("{}", format_species_table(metrics));
}

/// Format sampling statistics table as a string.
pub fn format_statistics_table(stats: &SamplingStatistics) -> String {
    let mut output = String::new();
    output.push_str(&format!("\n{}\n", "Sampling Statistics".bold().green()));
    output.push_str(&format!(
        "{}\n",
        format!(
            "Confidence Level: {:.0}% | Sample Size: {} plots",
            stats.tpa.confidence_level * 100.0,
            stats.tpa.sample_size
        )
        .dimmed()
    ));
    output.push_str(&format!("{}\n", "=".repeat(70)));

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            "Metric",
            "Mean",
            "Std Error",
            "Lower CI",
            "Upper CI",
            "Samp. Error %",
        ]);

    let metrics = [
        ("TPA", &stats.tpa),
        ("Basal Area (sq ft/ac)", &stats.basal_area),
        ("Volume (cu ft/ac)", &stats.volume_cuft),
        ("Volume (bd ft/ac)", &stats.volume_bdft),
    ];

    for (name, ci) in &metrics {
        table.add_row(vec![
            Cell::new(name),
            Cell::new(format!("{:.1}", ci.mean)),
            Cell::new(format!("{:.2}", ci.std_error)),
            Cell::new(format!("{:.1}", ci.lower)),
            Cell::new(format!("{:.1}", ci.upper)),
            Cell::new(format!("{:.1}%", ci.sampling_error_percent)),
        ]);
    }

    output.push_str(&format!("{table}"));
    output
}

/// Print sampling statistics table with confidence intervals.
pub fn print_statistics_table(stats: &SamplingStatistics) {
    print!("{}", format_statistics_table(stats));
}

/// Format growth projection table as a string.
pub fn format_growth_table(projections: &[GrowthProjection]) -> String {
    let mut output = String::new();
    output.push_str(&format!("\n{}\n", "Growth Projections".bold().green()));
    output.push_str(&format!("{}\n", "=".repeat(60)));

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["Year", "TPA", "BA/ac", "Vol (cuft/ac)", "Vol (bdft/ac)"]);

    for proj in projections {
        table.add_row(vec![
            Cell::new(format!("{}", proj.year)),
            Cell::new(format!("{:.1}", proj.tpa)),
            Cell::new(format!("{:.1}", proj.basal_area)),
            Cell::new(format!("{:.1}", proj.volume_cuft)),
            Cell::new(format!("{:.0}", proj.volume_bdft)),
        ]);
    }

    output.push_str(&format!("{table}"));
    output
}

/// Print growth projection table.
pub fn print_growth_table(projections: &[GrowthProjection]) {
    print!("{}", format_growth_table(projections));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::{
        compute_stand_metrics, ConfidenceInterval, GrowthProjection, SamplingStatistics,
    };
    use crate::models::{ForestInventory, Plot, Species, Tree, TreeStatus};

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
        let mut inv = ForestInventory::new("Viz Test");
        inv.plots
            .push(make_plot(1, vec![make_tree(1, 14.0), make_tree(1, 16.0)]));
        inv.plots
            .push(make_plot(2, vec![make_tree(2, 12.0), make_tree(2, 18.0)]));
        inv
    }

    fn sample_ci() -> ConfidenceInterval {
        ConfidenceInterval {
            mean: 10.0,
            std_error: 1.0,
            lower: 8.0,
            upper: 12.0,
            confidence_level: 0.95,
            sample_size: 5,
            sampling_error_percent: 20.0,
        }
    }

    #[test]
    fn test_format_stand_summary_contains_metrics() {
        let inv = sample_inventory();
        let metrics = compute_stand_metrics(&inv);
        let output = format_stand_summary(&metrics);
        assert!(output.contains("Trees per Acre"));
        assert!(output.contains("Basal Area"));
        assert!(output.contains("QMD"));
        assert!(output.contains("Number of Species"));
    }

    #[test]
    fn test_format_stand_summary_with_height() {
        let inv = sample_inventory();
        let metrics = compute_stand_metrics(&inv);
        let output = format_stand_summary(&metrics);
        assert!(output.contains("Mean Height"));
    }

    #[test]
    fn test_format_species_table_contains_headers() {
        let inv = sample_inventory();
        let metrics = compute_stand_metrics(&inv);
        let output = format_species_table(&metrics);
        assert!(output.contains("Species"));
        assert!(output.contains("Code"));
        assert!(output.contains("TPA"));
        assert!(output.contains("BA/ac"));
    }

    #[test]
    fn test_format_species_table_contains_species_data() {
        let inv = sample_inventory();
        let metrics = compute_stand_metrics(&inv);
        let output = format_species_table(&metrics);
        assert!(output.contains("Douglas Fir"));
        assert!(output.contains("DF"));
    }

    #[test]
    fn test_format_statistics_table_contains_fields() {
        let stats = SamplingStatistics {
            tpa: sample_ci(),
            basal_area: sample_ci(),
            volume_cuft: sample_ci(),
            volume_bdft: sample_ci(),
        };
        let output = format_statistics_table(&stats);
        assert!(output.contains("TPA"));
        assert!(output.contains("Basal Area"));
        assert!(output.contains("Mean"));
        assert!(output.contains("Std Error"));
        assert!(output.contains("Lower CI"));
        assert!(output.contains("Upper CI"));
    }

    #[test]
    fn test_format_growth_table_contains_headers() {
        let projections = vec![
            GrowthProjection {
                year: 0,
                tpa: 100.0,
                basal_area: 50.0,
                volume_cuft: 1000.0,
                volume_bdft: 5000.0,
            },
            GrowthProjection {
                year: 5,
                tpa: 98.0,
                basal_area: 55.0,
                volume_cuft: 1100.0,
                volume_bdft: 5500.0,
            },
        ];
        let output = format_growth_table(&projections);
        assert!(output.contains("Year"));
        assert!(output.contains("TPA"));
        assert!(output.contains("BA/ac"));
        assert!(output.contains("Vol (cuft/ac)"));
        assert!(output.contains("Vol (bdft/ac)"));
    }

    #[test]
    fn test_format_growth_table_contains_data() {
        let projections = vec![GrowthProjection {
            year: 10,
            tpa: 95.0,
            basal_area: 60.0,
            volume_cuft: 1200.0,
            volume_bdft: 6000.0,
        }];
        let output = format_growth_table(&projections);
        assert!(output.contains("10"));
        assert!(output.contains("95.0"));
    }

    #[test]
    fn test_format_growth_table_empty() {
        let output = format_growth_table(&[]);
        assert!(output.contains("Growth Projections"));
    }
}
