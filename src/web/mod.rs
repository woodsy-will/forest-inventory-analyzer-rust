//! Optional web server providing a REST API and embedded dashboard for forest inventory analysis.
//!
//! Gated behind the `web` feature flag. Supports file upload, validation, metrics computation,
//! growth projections, and data export through HTTP endpoints powered by Actix Web.

mod handlers;
mod state;

use actix_cors::Cors;
use actix_web::{http::header, web, App, HttpServer};
use state::AppState;
use tracing_actix_web::TracingLogger;

use crate::config::AppConfig;

pub async fn start_server(config: AppConfig) -> std::io::Result<()> {
    let port = config.server.port;
    let max_upload = config.server.max_upload_bytes;
    let bind_addr = config.server.bind_address.clone();

    let state =
        AppState::new(&config.database.path).map_err(|e| std::io::Error::other(e.to_string()))?;
    let data = web::Data::new(state);
    let upload_limit = web::Data::new(max_upload);

    tracing::info!("Starting Forest Inventory Analyzer web server on http://{bind_addr}:{port}");

    let server = HttpServer::new(move || {
        let multipart_cfg =
            actix_multipart::form::MultipartFormConfig::default().total_limit(max_upload);
        let payload_cfg = web::PayloadConfig::new(max_upload);
        let json_cfg = web::JsonConfig::default().limit(max_upload);

        let cors = Cors::default()
            .allowed_origin(&format!("http://localhost:{port}"))
            .allowed_origin(&format!("http://127.0.0.1:{port}"))
            .allowed_origin_fn(|origin, _req| {
                let s = origin.to_str().unwrap_or("");
                // Allow any origin for local/LAN access in desktop mode
                // RFC 1918 private ranges: 192.168.0.0/16, 10.0.0.0/8, 172.16.0.0/12
                s.starts_with("http://192.168.")
                    || s.starts_with("http://10.")
                    || s.strip_prefix("http://172.").is_some_and(|rest| {
                        rest.split('.')
                            .next()
                            .and_then(|octet| octet.parse::<u8>().ok())
                            .is_some_and(|o| (16..=31).contains(&o))
                    })
            })
            .allowed_methods(vec!["GET", "POST"])
            .allowed_header(header::CONTENT_TYPE)
            .max_age(3600);

        App::new()
            .wrap(TracingLogger::default())
            .wrap(cors)
            .app_data(data.clone())
            .app_data(upload_limit.clone())
            .app_data(multipart_cfg)
            .app_data(payload_cfg)
            .app_data(json_cfg)
            // Health check
            .route("/health", web::get().to(handlers::health))
            // Static files
            .route("/", web::get().to(handlers::index_html))
            .route("/app.js", web::get().to(handlers::app_js))
            .route("/style.css", web::get().to(handlers::style_css))
            .route("/chart.min.js", web::get().to(handlers::chart_js))
            // API routes
            .route("/api/upload", web::post().to(handlers::upload))
            .route(
                "/api/validate",
                web::post().to(handlers::validate_and_submit),
            )
            .route("/api/autofix", web::post().to(handlers::autofix))
            .route("/api/{id}/metrics", web::get().to(handlers::metrics))
            .route("/api/{id}/statistics", web::get().to(handlers::statistics))
            .route(
                "/api/{id}/distribution",
                web::get().to(handlers::distribution),
            )
            .route("/api/{id}/growth", web::post().to(handlers::growth))
            .route("/api/{id}/export", web::get().to(handlers::export))
            .route(
                "/api/{id}/inventory",
                web::get().to(handlers::inventory_json),
            )
    })
    .bind((&*bind_addr, port))
    .map_err(|e| {
        if e.kind() == std::io::ErrorKind::AddrInUse {
            std::io::Error::new(
                std::io::ErrorKind::AddrInUse,
                format!("Port {port} is already in use. Change port in config.toml or use --port flag.")
            )
        } else {
            e
        }
    })?
    .run();

    // Graceful shutdown: spawn a task that listens for ctrl-c
    let handle = server.handle();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        tracing::info!("Shutdown signal received, draining connections...");
        handle.stop(true).await;
    });

    server.await?;
    tracing::info!("Server has shut down");
    Ok(())
}
