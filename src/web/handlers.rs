use actix_multipart::Multipart;
use actix_web::{web, HttpResponse};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::analysis::{Analyzer, GrowthModel};
use crate::error::ForestError;
use crate::io;

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
            ForestError::InsufficientData(_) => {
                (actix_web::http::StatusCode::UNPROCESSABLE_ENTITY, "Unprocessable Entity")
            }
            _ => {
                (actix_web::http::StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error")
            }
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
    species: Vec<String>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn upload(
    state: web::Data<AppState>,
    mut payload: Multipart,
) -> Result<HttpResponse, WebError> {
    while let Some(Ok(mut field)) = payload.next().await {
        let filename = field
            .content_disposition()
            .and_then(|cd| cd.get_filename().map(|s| s.to_string()))
            .unwrap_or_else(|| "unknown".to_string());

        let mut bytes = Vec::new();
        while let Some(Ok(chunk)) = field.next().await {
            bytes.extend_from_slice(&chunk);
        }

        let ext = filename
            .rsplit('.')
            .next()
            .unwrap_or("")
            .to_lowercase();

        let name = filename
            .rsplit('.')
            .nth(1)
            .unwrap_or(&filename)
            .to_string();

        let inventory = match ext.as_str() {
            "csv" => io::read_csv_from_bytes(&bytes, &name)?,
            "json" => io::read_json_from_bytes(&bytes, &name)?,
            "xlsx" | "xls" => io::read_excel_from_bytes(&bytes, &name)?,
            _ => {
                return Ok(HttpResponse::BadRequest().json(ErrorBody {
                    error: "Bad Request".to_string(),
                    details: format!("Unsupported file format: .{ext}. Use .csv, .json, or .xlsx"),
                }));
            }
        };

        let id = Uuid::new_v4();
        let resp = UploadResponse {
            id,
            name: inventory.name.clone(),
            num_plots: inventory.num_plots(),
            num_trees: inventory.num_trees(),
            species: inventory.species_list().into_iter().map(|s| s.common_name).collect(),
        };

        state.inventories.lock().unwrap().insert(id, inventory);

        return Ok(HttpResponse::Ok().json(resp));
    }

    Ok(HttpResponse::BadRequest().json(ErrorBody {
        error: "Bad Request".to_string(),
        details: "No file uploaded".to_string(),
    }))
}

pub async fn metrics(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, WebError> {
    let id = path.into_inner();
    let inventory = state.get_inventory(&id).ok_or_else(|| {
        WebError(ForestError::ParseError(format!("Inventory {id} not found")))
    })?;
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
    let inventory = state.get_inventory(&id).ok_or_else(|| {
        WebError(ForestError::ParseError(format!("Inventory {id} not found")))
    })?;
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
    let inventory = state.get_inventory(&id).ok_or_else(|| {
        WebError(ForestError::ParseError(format!("Inventory {id} not found")))
    })?;
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
    let inventory = state.get_inventory(&id).ok_or_else(|| {
        WebError(ForestError::ParseError(format!("Inventory {id} not found")))
    })?;
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
    let inventory = state.get_inventory(&id).ok_or_else(|| {
        WebError(ForestError::ParseError(format!("Inventory {id} not found")))
    })?;
    let fmt = query.format.as_deref().unwrap_or("csv");

    match fmt {
        "csv" => {
            let mut wtr = csv::Writer::from_writer(Vec::new());
            for plot in &inventory.plots {
                for tree in &plot.trees {
                    wtr.serialize(&CsvExportRow::from_tree(tree, plot))
                        .map_err(|e| WebError(ForestError::Csv(e)))?;
                }
            }
            let data = wtr.into_inner().map_err(|e| WebError(ForestError::Io(
                std::io::Error::new(std::io::ErrorKind::Other, e.to_string()),
            )))?;
            Ok(HttpResponse::Ok()
                .content_type("text/csv")
                .insert_header(("Content-Disposition", format!("attachment; filename=\"{}.csv\"", inventory.name)))
                .body(data))
        }
        "json" => {
            let data = serde_json::to_string_pretty(&inventory)
                .map_err(|e| WebError(ForestError::Json(e)))?;
            Ok(HttpResponse::Ok()
                .content_type("application/json")
                .insert_header(("Content-Disposition", format!("attachment; filename=\"{}.json\"", inventory.name)))
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
    let inventory = state.get_inventory(&id).ok_or_else(|| {
        WebError(ForestError::ParseError(format!("Inventory {id} not found")))
    })?;
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
