use colored::Colorize;
use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Cell, ContentArrangement, Table};

use crate::analysis::{GrowthProjection, SamplingStatistics, StandMetrics};

/// Print a formatted stand summary table.
pub fn print_stand_summary(metrics: &StandMetrics) {
    println!("\n{}", "Stand Summary".bold().green());
    println!("{}", "=".repeat(50));

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

    println!("{table}");
}

/// Print species composition table.
pub fn print_species_table(metrics: &StandMetrics) {
    println!("\n{}", "Species Composition".bold().green());
    println!("{}", "=".repeat(50));

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

    println!("{table}");
}

/// Print sampling statistics table with confidence intervals.
pub fn print_statistics_table(stats: &SamplingStatistics) {
    println!("\n{}", "Sampling Statistics".bold().green());
    println!(
        "{}",
        format!(
            "Confidence Level: {:.0}% | Sample Size: {} plots",
            stats.tpa.confidence_level * 100.0,
            stats.tpa.sample_size
        )
        .dimmed()
    );
    println!("{}", "=".repeat(70));

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

    println!("{table}");
}

/// Print growth projection table.
pub fn print_growth_table(projections: &[GrowthProjection]) {
    println!("\n{}", "Growth Projections".bold().green());
    println!("{}", "=".repeat(60));

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

    println!("{table}");
}
