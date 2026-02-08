use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;

use forest_inventory_analyzer::{
    analysis::{
        compute_stand_metrics, project_growth, DiameterDistribution, GrowthModel,
        SamplingStatistics,
    },
    io,
    visualization::{
        print_diameter_histogram, print_growth_table, print_species_table, print_stand_summary,
        print_statistics_table,
    },
};

#[derive(Parser)]
#[command(
    name = "forest-analyzer",
    about = "Forest Inventory Analyzer - Comprehensive stand analysis tool",
    version,
    author
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze forest inventory data and display stand metrics
    Analyze {
        /// Path to input file (CSV, JSON, or Excel)
        #[arg(short, long)]
        input: PathBuf,

        /// Confidence level for statistical analysis (0.0-1.0)
        #[arg(short, long, default_value = "0.95")]
        confidence: f64,

        /// Diameter class width in inches for distribution
        #[arg(short, long, default_value = "2.0")]
        diameter_class_width: f64,

        /// Show detailed species composition
        #[arg(long, default_value = "true")]
        species: bool,

        /// Show diameter distribution histogram
        #[arg(long, default_value = "true")]
        distribution: bool,
    },

    /// Project stand growth over time
    Growth {
        /// Path to input file (CSV, JSON, or Excel)
        #[arg(short, long)]
        input: PathBuf,

        /// Number of years to project
        #[arg(short, long, default_value = "20")]
        years: u32,

        /// Growth model: exponential, logistic, or linear
        #[arg(short, long, default_value = "logistic")]
        model: String,

        /// Annual growth rate (for exponential/logistic models)
        #[arg(short, long, default_value = "0.03")]
        rate: f64,

        /// Carrying capacity for basal area (logistic model, sq ft/acre)
        #[arg(short, long, default_value = "300.0")]
        capacity: f64,

        /// Annual mortality rate (proportion for exponential/logistic, TPA/year for linear)
        #[arg(long)]
        mortality: Option<f64>,
    },

    /// Convert inventory data between formats
    Convert {
        /// Input file path
        #[arg(short, long)]
        input: PathBuf,

        /// Output file path
        #[arg(short, long)]
        output: PathBuf,

        /// Pretty-print JSON output
        #[arg(long)]
        pretty: bool,
    },

    /// Display a quick summary of the inventory
    Summary {
        /// Path to input file
        #[arg(short, long)]
        input: PathBuf,
    },

    /// Start the web UI server
    #[cfg(feature = "web")]
    Serve {
        /// Port to listen on
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },
}

fn load_inventory(path: &PathBuf) -> Result<forest_inventory_analyzer::models::ForestInventory> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "csv" => Ok(io::read_csv(path)?),
        "json" => Ok(io::read_json(path)?),
        "xlsx" | "xls" => Ok(io::read_excel(path)?),
        _ => anyhow::bail!("Unsupported file format: .{ext}. Use .csv, .json, or .xlsx"),
    }
}

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Analyze {
            input,
            confidence,
            diameter_class_width,
            species,
            distribution,
        } => {
            println!(
                "\n{}",
                format!("Forest Inventory Analysis: {}", input.display())
                    .bold()
                    .cyan()
            );

            let inventory = load_inventory(&input)?;
            println!(
                "  Loaded {} plots with {} trees",
                inventory.num_plots(),
                inventory.num_trees()
            );

            let metrics = compute_stand_metrics(&inventory);
            print_stand_summary(&metrics);

            if species {
                print_species_table(&metrics);
            }

            if distribution {
                let dist = DiameterDistribution::from_inventory(&inventory, diameter_class_width);
                print_diameter_histogram(&dist);
            }

            match SamplingStatistics::compute(&inventory, confidence) {
                Ok(stats) => print_statistics_table(&stats),
                Err(e) => {
                    eprintln!("{}: {e}", "Warning".yellow());
                }
            }
        }

        Commands::Growth {
            input,
            years,
            model,
            rate,
            capacity,
            mortality,
        } => {
            let inventory = load_inventory(&input)?;

            let growth_model = match model.to_lowercase().as_str() {
                "exponential" | "exp" => GrowthModel::Exponential {
                    annual_rate: rate,
                    mortality_rate: mortality.unwrap_or(0.005),
                },
                "logistic" | "log" => GrowthModel::Logistic {
                    annual_rate: rate,
                    carrying_capacity: capacity,
                    mortality_rate: mortality.unwrap_or(0.005),
                },
                "linear" | "lin" => GrowthModel::Linear {
                    annual_increment: rate,
                    mortality_rate: mortality.unwrap_or(0.5),
                },
                _ => anyhow::bail!(
                    "Unknown growth model: {model}. Use: exponential, logistic, or linear"
                ),
            };

            println!(
                "\n{}",
                format!("Growth Projection: {} years ({model})", years)
                    .bold()
                    .cyan()
            );

            let projections = project_growth(&inventory, &growth_model, years)?;
            print_growth_table(&projections);
        }

        Commands::Convert {
            input,
            output,
            pretty,
        } => {
            let inventory = load_inventory(&input)?;

            let out_ext = output
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();

            match out_ext.as_str() {
                "csv" => io::write_csv(&inventory, &output)?,
                "json" => io::write_json(&inventory, &output, pretty)?,
                "xlsx" => io::write_excel(&inventory, &output)?,
                _ => anyhow::bail!("Unsupported output format: .{out_ext}"),
            }

            println!(
                "{} Converted {} -> {}",
                "Success:".green().bold(),
                input.display(),
                output.display()
            );
        }

        Commands::Summary { input } => {
            let inventory = load_inventory(&input)?;

            println!("\n{}", "Quick Summary".bold().cyan());
            println!("{}", "=".repeat(40));
            println!("  Name:           {}", inventory.name);
            println!("  Plots:          {}", inventory.num_plots());
            println!("  Total Trees:    {}", inventory.num_trees());
            println!("  Species:        {}", inventory.species_list().len());
            println!("  Mean TPA:       {:.1}", inventory.mean_tpa());
            println!("  Mean BA/ac:     {:.1} sq ft", inventory.mean_basal_area());
            println!(
                "  Mean Vol/ac:    {:.1} cu ft",
                inventory.mean_volume_cuft()
            );
            println!(
                "  Mean Vol/ac:    {:.0} bd ft",
                inventory.mean_volume_bdft()
            );
        }

        #[cfg(feature = "web")]
        Commands::Serve { port } => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(forest_inventory_analyzer::web::start_server(port))?;
        }
    }

    Ok(())
}
