use actix_multipart::Multipart;
use actix_web::{web, HttpResponse};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::analysis::{Analyzer, GrowthModel};
use crate::error::ForestError;
use crate::io::{self, rows_to_inventory, EditableTreeRow};
use crate::models::{Species, Tree, TreeStatus, ValidationIssue};

use super::state::AppState;

// ---------------------------------------------------------------------------
// Error wrapper
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: String,
    details: String,
}

#[derive(Debug)]
pub(crate) struct WebError(ForestError);

impl From<ForestError> for WebError {
    fn from(e: ForestError) -> Self {
        WebError(e)
    }
}

impl std::fmt::Display for WebError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl actix_web::ResponseError for WebError {
    fn error_response(&self) -> HttpResponse {
        let (status, error_type) = match &self.0 {
            ForestError::ValidationError(_) | ForestError::ParseError(_) => {
                (actix_web::http::StatusCode::BAD_REQUEST, "Bad Request")
            }
            ForestError::NotFound(_) => (actix_web::http::StatusCode::NOT_FOUND, "Not Found"),
            ForestError::InsufficientData(_) => (
                actix_web::http::StatusCode::UNPROCESSABLE_ENTITY,
                "Unprocessable Entity",
            ),
            _ => (
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Internal Server Error",
            ),
        };
        HttpResponse::build(status).json(ErrorBody {
            error: error_type.to_string(),
            details: self.0.to_string(),
        })
    }
}

// ---------------------------------------------------------------------------
// Upload response
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct UploadResponse {
    id: Uuid,
    name: String,
    num_plots: usize,
    num_trees: usize,
    has_errors: bool,
    errors: Vec<ValidationIssue>,
    trees: Vec<EditableTreeRow>,
    species: Vec<String>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Collect unique species names from editable rows.
fn species_from_rows(rows: &[EditableTreeRow]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut species = Vec::new();
    for row in rows {
        if seen.insert(row.species_name.clone()) {
            species.push(row.species_name.clone());
        }
    }
    species
}

/// Count distinct plot IDs in editable rows.
fn num_plots_from_rows(rows: &[EditableTreeRow]) -> usize {
    rows.iter()
        .map(|r| r.plot_id)
        .collect::<std::collections::HashSet<_>>()
        .len()
}

/// Sanitize a filename for use in Content-Disposition headers.
/// Removes characters that could enable header injection or path traversal.
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.' || *c == ' ')
        .collect::<String>()
        .replace("..", "")
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn upload(
    state: web::Data<AppState>,
    mut payload: Multipart,
) -> Result<HttpResponse, WebError> {
    if let Some(Ok(mut field)) = payload.next().await {
        let filename = field
            .content_disposition()
            .and_then(|cd| cd.get_filename().map(|s| s.to_string()))
            .unwrap_or_else(|| "unknown".to_string());

        let mut bytes = Vec::new();
        while let Some(Ok(chunk)) = field.next().await {
            bytes.extend_from_slice(&chunk);
        }

        let path = std::path::Path::new(&filename);
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(&filename)
            .to_string();

        let (inv_name, rows, issues) = match ext.as_str() {
            "csv" => io::parse_csv_lenient(&bytes, &name)?,
            "json" => io::parse_json_lenient(&bytes, &name)?,
            "xlsx" | "xls" => io::parse_excel_lenient(&bytes, &name)?,
            _ => {
                return Ok(HttpResponse::BadRequest().json(ErrorBody {
                    error: "Bad Request".to_string(),
                    details: format!("Unsupported file format: .{ext}. Use .csv, .json, or .xlsx"),
                }));
            }
        };

        let id = Uuid::new_v4();
        let has_errors = !issues.is_empty();

        if has_errors {
            // Store pending rows for later revalidation
            let resp = UploadResponse {
                id,
                name: inv_name.clone(),
                num_plots: num_plots_from_rows(&rows),
                num_trees: rows.len(),
                has_errors: true,
                errors: issues,
                trees: rows.clone(),
                species: species_from_rows(&rows),
            };
            state.insert_pending(id, inv_name, rows);
            return Ok(HttpResponse::Ok().json(resp));
        } else {
            // No errors — build inventory and store it
            let inventory = rows_to_inventory(&inv_name, &rows);
            let resp = UploadResponse {
                id,
                name: inventory.name.clone(),
                num_plots: inventory.num_plots(),
                num_trees: inventory.num_trees(),
                has_errors: false,
                errors: vec![],
                trees: vec![],
                species: inventory
                    .species_list()
                    .into_iter()
                    .map(|s| s.common_name)
                    .collect(),
            };
            state.insert_inventory(id, inventory);
            return Ok(HttpResponse::Ok().json(resp));
        }
    }

    Ok(HttpResponse::BadRequest().json(ErrorBody {
        error: "Bad Request".to_string(),
        details: "No file uploaded".to_string(),
    }))
}

// ---------------------------------------------------------------------------
// Validate & submit endpoint
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct ValidateRequest {
    id: Uuid,
    trees: Vec<EditableTreeRow>,
}

pub async fn validate_and_submit(
    state: web::Data<AppState>,
    body: web::Json<ValidateRequest>,
) -> Result<HttpResponse, WebError> {
    // Reject requests for unknown IDs — must come from a prior upload
    if !state.has_pending(&body.id) {
        return Ok(HttpResponse::NotFound().json(ErrorBody {
            error: "Not Found".to_string(),
            details: format!("No pending upload found for id {}", body.id),
        }));
    }

    let mut all_issues = Vec::new();

    for row in &body.trees {
        // Check status validity
        if row.status.parse::<TreeStatus>().is_err() {
            all_issues.push(ValidationIssue {
                plot_id: row.plot_id,
                tree_id: row.tree_id,
                row_index: row.row_index,
                field: "status".to_string(),
                message: format!("Unknown tree status '{}'", row.status),
            });
        }

        // Build a Tree to validate
        let status: TreeStatus = row.status.parse().unwrap_or(TreeStatus::Live);
        let tree = Tree {
            tree_id: row.tree_id,
            plot_id: row.plot_id,
            species: Species {
                code: row.species_code.clone(),
                common_name: row.species_name.clone(),
            },
            dbh: row.dbh,
            height: row.height,
            crown_ratio: row.crown_ratio,
            status,
            expansion_factor: row.expansion_factor,
            age: row.age,
            defect: row.defect,
        };

        all_issues.extend(tree.validate_all(row.row_index));
    }

    let has_errors = !all_issues.is_empty();

    if has_errors {
        // Update pending rows, preserving the original name
        let name = state
            .get_pending_name(&body.id)
            .unwrap_or_else(|| "Unknown".to_string());
        state.insert_pending(body.id, name.clone(), body.trees.clone());

        let resp = UploadResponse {
            id: body.id,
            name,
            num_plots: num_plots_from_rows(&body.trees),
            num_trees: body.trees.len(),
            has_errors: true,
            errors: all_issues,
            trees: body.trees.clone(),
            species: species_from_rows(&body.trees),
        };
        Ok(HttpResponse::Ok().json(resp))
    } else {
        // Clean — build inventory, move from pending to inventories
        let name = state
            .remove_pending(&body.id)
            .map(|(n, _)| n)
            .unwrap_or_else(|| "Unknown".to_string());
        let inventory = rows_to_inventory(&name, &body.trees);
        let resp = UploadResponse {
            id: body.id,
            name: inventory.name.clone(),
            num_plots: inventory.num_plots(),
            num_trees: inventory.num_trees(),
            has_errors: false,
            errors: vec![],
            trees: vec![],
            species: inventory
                .species_list()
                .into_iter()
                .map(|s| s.common_name)
                .collect(),
        };
        state.insert_inventory(body.id, inventory);
        Ok(HttpResponse::Ok().json(resp))
    }
}

pub async fn metrics(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, WebError> {
    let id = path.into_inner();
    let inventory = state
        .get_inventory(&id)
        .ok_or_else(|| WebError(ForestError::NotFound(format!("Inventory {id} not found"))))?;
    let analyzer = Analyzer::new(&inventory);
    Ok(HttpResponse::Ok().json(analyzer.stand_metrics()))
}

#[derive(Deserialize)]
pub struct StatsQuery {
    confidence: Option<f64>,
}

pub async fn statistics(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    query: web::Query<StatsQuery>,
) -> Result<HttpResponse, WebError> {
    let id = path.into_inner();
    let inventory = state
        .get_inventory(&id)
        .ok_or_else(|| WebError(ForestError::NotFound(format!("Inventory {id} not found"))))?;
    let confidence = query.confidence.unwrap_or(0.95);
    let analyzer = Analyzer::new(&inventory);
    let stats = analyzer.sampling_statistics(confidence)?;
    Ok(HttpResponse::Ok().json(stats))
}

#[derive(Deserialize)]
pub struct DistQuery {
    class_width: Option<f64>,
}

pub async fn distribution(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    query: web::Query<DistQuery>,
) -> Result<HttpResponse, WebError> {
    let id = path.into_inner();
    let inventory = state
        .get_inventory(&id)
        .ok_or_else(|| WebError(ForestError::NotFound(format!("Inventory {id} not found"))))?;
    let class_width = query.class_width.unwrap_or(2.0);
    let analyzer = Analyzer::new(&inventory);
    Ok(HttpResponse::Ok().json(analyzer.diameter_distribution(class_width)))
}

#[derive(Deserialize)]
pub struct GrowthRequest {
    model: GrowthModel,
    years: u32,
}

pub async fn growth(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<GrowthRequest>,
) -> Result<HttpResponse, WebError> {
    let id = path.into_inner();
    let inventory = state
        .get_inventory(&id)
        .ok_or_else(|| WebError(ForestError::NotFound(format!("Inventory {id} not found"))))?;
    let analyzer = Analyzer::new(&inventory);
    let projections = analyzer.project_growth(&body.model, body.years)?;
    Ok(HttpResponse::Ok().json(projections))
}

#[derive(Deserialize)]
pub struct ExportQuery {
    format: Option<String>,
}

pub async fn export(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    query: web::Query<ExportQuery>,
) -> Result<HttpResponse, WebError> {
    let id = path.into_inner();
    let inventory = state
        .get_inventory(&id)
        .ok_or_else(|| WebError(ForestError::NotFound(format!("Inventory {id} not found"))))?;
    let fmt = query.format.as_deref().unwrap_or("csv");

    match fmt {
        "csv" => {
            let mut wtr = csv::Writer::from_writer(Vec::new());
            for plot in &inventory.plots {
                for tree in &plot.trees {
                    wtr.serialize(CsvExportRow::from_tree(tree, plot))
                        .map_err(|e| WebError(ForestError::Csv(e)))?;
                }
            }
            let data = wtr
                .into_inner()
                .map_err(|e| WebError(ForestError::Io(std::io::Error::other(e.to_string()))))?;
            let safe_name = sanitize_filename(&inventory.name);
            Ok(HttpResponse::Ok()
                .content_type("text/csv")
                .insert_header((
                    "Content-Disposition",
                    format!("attachment; filename=\"{}.csv\"", safe_name),
                ))
                .body(data))
        }
        "json" => {
            let data = serde_json::to_string_pretty(&inventory)
                .map_err(|e| WebError(ForestError::Json(e)))?;
            let safe_name = sanitize_filename(&inventory.name);
            Ok(HttpResponse::Ok()
                .content_type("application/json")
                .insert_header((
                    "Content-Disposition",
                    format!("attachment; filename=\"{}.json\"", safe_name),
                ))
                .body(data))
        }
        _ => Ok(HttpResponse::BadRequest().json(ErrorBody {
            error: "Bad Request".to_string(),
            details: format!("Unsupported export format: {fmt}. Use csv or json."),
        })),
    }
}

#[derive(serde::Serialize)]
struct CsvExportRow {
    plot_id: u32,
    tree_id: u32,
    species_code: String,
    species_name: String,
    dbh: f64,
    height: Option<f64>,
    crown_ratio: Option<f64>,
    status: String,
    expansion_factor: f64,
    age: Option<u32>,
    defect: Option<f64>,
    plot_size_acres: f64,
    slope_percent: Option<f64>,
    aspect_degrees: Option<f64>,
    elevation_ft: Option<f64>,
}

impl CsvExportRow {
    fn from_tree(tree: &crate::models::Tree, plot: &crate::models::Plot) -> Self {
        Self {
            plot_id: tree.plot_id,
            tree_id: tree.tree_id,
            species_code: tree.species.code.clone(),
            species_name: tree.species.common_name.clone(),
            dbh: tree.dbh,
            height: tree.height,
            crown_ratio: tree.crown_ratio,
            status: tree.status.to_string(),
            expansion_factor: tree.expansion_factor,
            age: tree.age,
            defect: tree.defect,
            plot_size_acres: plot.plot_size_acres,
            slope_percent: plot.slope_percent,
            aspect_degrees: plot.aspect_degrees,
            elevation_ft: plot.elevation_ft,
        }
    }
}

pub async fn inventory_json(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, WebError> {
    let id = path.into_inner();
    let inventory = state
        .get_inventory(&id)
        .ok_or_else(|| WebError(ForestError::NotFound(format!("Inventory {id} not found"))))?;
    Ok(HttpResponse::Ok().json(inventory))
}

// ---------------------------------------------------------------------------
// Static file handlers
// ---------------------------------------------------------------------------

pub async fn index_html() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../../static/index.html"))
}

pub async fn app_js() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/javascript; charset=utf-8")
        .body(include_str!("../../static/app.js"))
}

pub async fn style_css() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/css; charset=utf-8")
        .body(include_str!("../../static/style.css"))
}
