use colored::Colorize;

use crate::analysis::DiameterDistribution;

/// Print a text-based histogram of the diameter distribution.
pub fn print_diameter_histogram(dist: &DiameterDistribution) {
    println!("\n{}", "Diameter Distribution".bold().green());
    println!("{}", "=".repeat(60));

    if dist.classes.is_empty() {
        println!("  No data available.");
        return;
    }

    let max_tpa = dist
        .classes
        .iter()
        .map(|c| c.tpa)
        .fold(0.0f64, f64::max);

    let bar_width = 40;

    println!(
        "  {:>10}  {:>8}  {:>8}  Distribution",
        "DBH Class", "TPA", "BA/ac"
    );
    println!("  {}", "-".repeat(70));

    for class in &dist.classes {
        let bar_len = if max_tpa > 0.0 {
            ((class.tpa / max_tpa) * bar_width as f64).round() as usize
        } else {
            0
        };

        let bar = "\u{2588}".repeat(bar_len);

        println!(
            "  {:>4.0}-{:<4.0}\"  {:>8.1}  {:>8.1}  {}",
            class.lower,
            class.upper,
            class.tpa,
            class.basal_area,
            bar.green()
        );
    }

    println!();
}
