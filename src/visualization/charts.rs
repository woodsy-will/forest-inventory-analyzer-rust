use colored::Colorize;

use crate::analysis::DiameterDistribution;

/// Format a text-based histogram of the diameter distribution as a string.
pub fn format_diameter_histogram(dist: &DiameterDistribution) -> String {
    let mut output = String::new();
    output.push_str(&format!("\n{}\n", "Diameter Distribution".bold().green()));
    output.push_str(&format!("{}\n", "=".repeat(60)));

    if dist.classes.is_empty() {
        output.push_str("  No data available.\n");
        return output;
    }

    let max_tpa = dist
        .classes
        .iter()
        .map(|c| c.tpa)
        .fold(0.0f64, f64::max);

    let bar_width = 40;

    output.push_str(&format!(
        "  {:>10}  {:>8}  {:>8}  Distribution\n",
        "DBH Class", "TPA", "BA/ac"
    ));
    output.push_str(&format!("  {}\n", "-".repeat(70)));

    for class in &dist.classes {
        let bar_len = if max_tpa > 0.0 {
            ((class.tpa / max_tpa) * bar_width as f64).round() as usize
        } else {
            0
        };

        let bar = "\u{2588}".repeat(bar_len);

        output.push_str(&format!(
            "  {:>4.0}-{:<4.0}\"  {:>8.1}  {:>8.1}  {}\n",
            class.lower,
            class.upper,
            class.tpa,
            class.basal_area,
            bar.green()
        ));
    }

    output.push('\n');
    output
}

/// Print a text-based histogram of the diameter distribution.
pub fn print_diameter_histogram(dist: &DiameterDistribution) {
    print!("{}", format_diameter_histogram(dist));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::{DiameterClass, DiameterDistribution};

    #[test]
    fn test_format_histogram_empty() {
        let dist = DiameterDistribution {
            class_width: 2.0,
            classes: vec![],
        };
        let output = format_diameter_histogram(&dist);
        assert!(output.contains("No data available."));
        assert!(output.contains("Diameter Distribution"));
    }

    #[test]
    fn test_format_histogram_with_data() {
        let dist = DiameterDistribution {
            class_width: 2.0,
            classes: vec![
                DiameterClass {
                    lower: 10.0,
                    upper: 12.0,
                    midpoint: 11.0,
                    tpa: 25.0,
                    basal_area: 15.0,
                    tree_count: 5,
                },
                DiameterClass {
                    lower: 12.0,
                    upper: 14.0,
                    midpoint: 13.0,
                    tpa: 15.0,
                    basal_area: 12.0,
                    tree_count: 3,
                },
            ],
        };
        let output = format_diameter_histogram(&dist);
        assert!(output.contains("DBH Class"));
        assert!(output.contains("TPA"));
        assert!(output.contains("BA/ac"));
        assert!(output.contains("Distribution"));
    }

    #[test]
    fn test_format_histogram_contains_values() {
        let dist = DiameterDistribution {
            class_width: 2.0,
            classes: vec![DiameterClass {
                lower: 14.0,
                upper: 16.0,
                midpoint: 15.0,
                tpa: 30.0,
                basal_area: 20.0,
                tree_count: 6,
            }],
        };
        let output = format_diameter_histogram(&dist);
        assert!(output.contains("30.0"));
        assert!(output.contains("20.0"));
    }
}
