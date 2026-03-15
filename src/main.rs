use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;

use forest_inventory_analyzer::{
    analysis::{
        compute_stand_metrics, project_growth, DiameterDistribution, GrowthModel,
        SamplingStatistics,
    },
    config::AppConfig,
    io,
    visualization::{
        print_diameter_histogram, print_growth_table, print_species_table, print_stand_summary,
        print_statistics_table,
    },
};

/// Supported input file extensions for inventory data.
const SUPPORTED_INPUT_EXTS: &[&str] = &["csv", "json", "xlsx", "xls"];

/// Parse and validate a confidence level in (0.0, 1.0) exclusive.
fn parse_confidence(s: &str) -> Result<f64, String> {
    let val: f64 = s
        .parse()
        .map_err(|_| format!("'{s}' is not a valid number"))?;
    if val <= 0.0 || val >= 1.0 {
        return Err(format!(
            "confidence must be between 0.0 and 1.0 exclusive, got {val}"
        ));
    }
    Ok(val)
}

/// Extract the lowercased file extension from a path, or empty string if none.
fn file_extension(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase()
}

/// Check whether a path has a supported inventory file extension.
fn is_supported_inventory_file(path: &Path) -> bool {
    let ext = file_extension(path);
    SUPPORTED_INPUT_EXTS.contains(&ext.as_str())
}

/// Load a forest inventory from a supported file format (CSV, JSON, Excel).
fn load_inventory(path: &Path) -> Result<forest_inventory_analyzer::models::ForestInventory> {
    let ext = file_extension(path);
    match ext.as_str() {
        "csv" => Ok(io::read_csv(path)?),
        "json" => Ok(io::read_json(path)?),
        "xlsx" | "xls" => Ok(io::read_excel(path)?),
        _ => anyhow::bail!("Unsupported file format: .{ext}. Use .csv, .json, or .xlsx"),
    }
}

/// Save a forest inventory to a supported output format (CSV, JSON, Excel, GeoJSON).
fn save_inventory(
    inventory: &forest_inventory_analyzer::models::ForestInventory,
    path: &Path,
    pretty: bool,
) -> Result<()> {
    let ext = file_extension(path);
    match ext.as_str() {
        "csv" => io::write_csv(inventory, path)?,
        "json" => io::write_json(inventory, path, pretty)?,
        "xlsx" => io::write_excel(inventory, path)?,
        "geojson" => io::write_geojson(inventory, path, pretty)?,
        _ => anyhow::bail!(
            "Unsupported output format: .{ext}. Use .csv, .json, .xlsx, or .geojson"
        ),
    }
    Ok(())
}

#[derive(Parser)]
#[command(
    name = "forest-analyzer",
    about = "Forest Inventory Analyzer - Comprehensive stand analysis tool",
    version,
    author
)]
struct Cli {
    /// Path to configuration file (default: config.toml)
    #[arg(long, global = true, default_value = "config.toml")]
    config: PathBuf,

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
        #[arg(short, long, default_value = "0.95", value_parser = parse_confidence)]
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

    /// Analyze multiple inventory files in a directory
    AnalyzeBatch {
        /// Directory containing inventory files (CSV, JSON, or Excel)
        #[arg(long)]
        input_dir: PathBuf,

        /// Directory for output JSON reports
        #[arg(long)]
        output_dir: PathBuf,

        /// Confidence level for statistical analysis (0.0-1.0)
        #[arg(short, long, default_value = "0.95", value_parser = parse_confidence)]
        confidence: f64,
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

        /// Address to bind the server to
        #[arg(short, long, default_value = "127.0.0.1")]
        bind: String,
    },
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let cli = Cli::parse();
    let _config = AppConfig::load(&cli.config)?;

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

            // Per-stand summaries for multi-stand cruise data
            let stands = inventory.stands();
            if !stands.is_empty() {
                println!(
                    "\n{}",
                    format!("Per-Stand Summary ({} stands)", stands.len())
                        .bold()
                        .cyan()
                );
                println!("{}", "=".repeat(72));
                for (stand_id, sub_inv) in &stands {
                    let sm = compute_stand_metrics(sub_inv);
                    println!(
                        "\n  {} ({} plots, {} trees)",
                        format!("Stand {stand_id}").bold(),
                        sub_inv.num_plots(),
                        sub_inv.num_trees()
                    );
                    println!(
                        "    TPA: {:.1}  |  BA: {:.1} ft\u{00B2}/ac  |  QMD: {:.1}\"  |  Vol: {:.0} bd ft/ac",
                        sm.total_tpa,
                        sm.total_basal_area,
                        sm.quadratic_mean_diameter,
                        sm.total_volume_bdft
                    );
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

            // Parse the model name into a GrowthModel with defaults, then
            // override individual fields with explicit CLI arguments.
            let mut growth_model: GrowthModel = model.parse().map_err(|e| {
                anyhow::anyhow!("{e}")
            })?;

            // Apply CLI overrides for rate/capacity/mortality
            match &mut growth_model {
                GrowthModel::Exponential {
                    annual_rate,
                    mortality_rate,
                } => {
                    *annual_rate = rate;
                    if let Some(m) = mortality {
                        *mortality_rate = m;
                    }
                }
                GrowthModel::Logistic {
                    annual_rate,
                    carrying_capacity,
                    mortality_rate,
                } => {
                    *annual_rate = rate;
                    *carrying_capacity = capacity;
                    if let Some(m) = mortality {
                        *mortality_rate = m;
                    }
                }
                GrowthModel::Linear {
                    annual_increment,
                    mortality_rate,
                } => {
                    *annual_increment = rate;
                    if let Some(m) = mortality {
                        *mortality_rate = m;
                    }
                }
            }

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
            save_inventory(&inventory, &output, pretty)?;

            println!(
                "{} Converted {} -> {}",
                "Success:".green().bold(),
                input.display(),
                output.display()
            );
        }

        Commands::AnalyzeBatch {
            input_dir,
            output_dir,
            confidence,
        } => {
            if !input_dir.is_dir() {
                anyhow::bail!("Input path is not a directory: {}", input_dir.display());
            }
            std::fs::create_dir_all(&output_dir)?;

            let mut files: Vec<PathBuf> = std::fs::read_dir(&input_dir)?
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .filter(|p| is_supported_inventory_file(p))
                .collect();
            files.sort();

            if files.is_empty() {
                anyhow::bail!(
                    "No inventory files (.csv, .json, .xlsx) found in {}",
                    input_dir.display()
                );
            }

            println!(
                "\n{}",
                format!("Batch Analysis: {} files", files.len())
                    .bold()
                    .cyan()
            );

            let mut processed = 0;
            let mut failed = 0;

            for file in &files {
                let name = file.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");
                match load_inventory(file) {
                    Ok(inventory) => {
                        let metrics = compute_stand_metrics(&inventory);
                        let stats = SamplingStatistics::compute(&inventory, confidence).ok();

                        let report = serde_json::json!({
                            "file": file.file_name().and_then(|f| f.to_str()),
                            "name": inventory.name,
                            "num_plots": inventory.num_plots(),
                            "num_trees": inventory.num_trees(),
                            "species_count": inventory.species_list().len(),
                            "mean_tpa": inventory.mean_tpa(),
                            "mean_basal_area": inventory.mean_basal_area(),
                            "mean_volume_cuft": inventory.mean_volume_cuft(),
                            "mean_volume_bdft": inventory.mean_volume_bdft(),
                            "species_composition": metrics.species_composition.iter().map(|sc| {
                                serde_json::json!({
                                    "species": sc.species.code,
                                    "tpa": sc.tpa,
                                    "basal_area": sc.basal_area,
                                    "pct_ba": sc.percent_basal_area,
                                })
                            }).collect::<Vec<_>>(),
                            "statistics": stats.map(|s| serde_json::json!({
                                "confidence_level": s.tpa.confidence_level,
                                "tpa_mean": s.tpa.mean,
                                "tpa_std_error": s.tpa.std_error,
                                "ba_mean": s.basal_area.mean,
                                "ba_std_error": s.basal_area.std_error,
                            })),
                        });

                        let out_path = output_dir.join(format!("{name}.json"));
                        let content = serde_json::to_string_pretty(&report)?;
                        std::fs::write(&out_path, content)?;

                        println!("  {} {}", "OK".green(), file.display());
                        processed += 1;
                    }
                    Err(e) => {
                        eprintln!("  {} {} — {e}", "FAIL".red(), file.display());
                        failed += 1;
                    }
                }
            }

            println!(
                "\n{} Processed {processed} files, {failed} failed. Reports in {}",
                "Done.".green().bold(),
                output_dir.display()
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
        Commands::Serve { port, bind } => {
            let mut server_config = _config;
            server_config.server.port = port;
            server_config.server.bind_address = bind;

            // Resolve relative database path relative to the executable's directory
            if !std::path::Path::new(&server_config.database.path).is_absolute() {
                if let Ok(exe_dir) = std::env::current_exe().map(|p| p.parent().unwrap().to_path_buf()) {
                    server_config.database.path = exe_dir.join(&server_config.database.path).to_string_lossy().to_string();
                }
            }

            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(forest_inventory_analyzer::web::start_server(server_config))?;
        }
    }

    Ok(())
}
