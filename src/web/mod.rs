mod handlers;
mod state;

use actix_web::{web, App, HttpServer};
use state::AppState;

pub async fn start_server(port: u16) -> std::io::Result<()> {
    let data = web::Data::new(AppState::new());

    println!("Starting Forest Inventory Analyzer web server on http://localhost:{port}");

    HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            // Static files
            .route("/", web::get().to(handlers::index_html))
            .route("/app.js", web::get().to(handlers::app_js))
            .route("/style.css", web::get().to(handlers::style_css))
            // API routes
            .route("/api/upload", web::post().to(handlers::upload))
            .route("/api/{id}/metrics", web::get().to(handlers::metrics))
            .route("/api/{id}/statistics", web::get().to(handlers::statistics))
            .route("/api/{id}/distribution", web::get().to(handlers::distribution))
            .route("/api/{id}/growth", web::post().to(handlers::growth))
            .route("/api/{id}/export", web::get().to(handlers::export))
            .route("/api/{id}/inventory", web::get().to(handlers::inventory_json))
    })
    .bind(("127.0.0.1", port))?
    .run()
    .await
}
