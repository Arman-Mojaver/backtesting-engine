use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use chrono::Local;
use log::info;
use std::io;

async fn health_check() -> impl Responder {
    HttpResponse::Ok().json({
        let now = Local::now();
        serde_json::json!({
            "status": "ok",
            "timestamp": now.to_rfc3339(),
        })
    })
}

fn configure_routes(config: &mut web::ServiceConfig) {
    config.route("/health", web::get().to(health_check));
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0:80".into());
    info!("Server starting on {}", &host);

    HttpServer::new(|| {
        App::new()
            .wrap(actix_web::middleware::Logger::default())
            .configure(configure_routes)
    })
        .workers(7)
        .bind(&host)?
        .run()
        .await
}
