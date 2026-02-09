mod handlers;
mod state;

use actix_web::{web, App, HttpServer};
use state::AppState;

/// Maximum upload size: 50 MB
const MAX_UPLOAD_SIZE: usize = 50 * 1024 * 1024;

pub async fn start_server(port: u16) -> std::io::Result<()> {
    let data = web::Data::new(AppState::new());

    println!("Starting Forest Inventory Analyzer web server on http://localhost:{port}");

    HttpServer::new(move || {
        let multipart_cfg =
            actix_multipart::form::MultipartFormConfig::default().total_limit(MAX_UPLOAD_SIZE);
        let payload_cfg = web::PayloadConfig::new(MAX_UPLOAD_SIZE);
        let json_cfg = web::JsonConfig::default().limit(MAX_UPLOAD_SIZE);

        App::new()
            .app_data(data.clone())
            .app_data(multipart_cfg)
            .app_data(payload_cfg)
            .app_data(json_cfg)
            // Static files
            .route("/", web::get().to(handlers::index_html))
            .route("/app.js", web::get().to(handlers::app_js))
            .route("/style.css", web::get().to(handlers::style_css))
            // API routes
            .route("/api/upload", web::post().to(handlers::upload))
            .route(
                "/api/validate",
                web::post().to(handlers::validate_and_submit),
            )
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
    .bind(("127.0.0.1", port))?
    .run()
    .await
}
