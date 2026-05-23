<!-- Context: development/backend/rust/actix-patterns | Priority: high | Version: 1.0 | Updated: 2026-05-23 -->

# Actix-web Patterns

**Stack**: actix-web 4.x · Rust 2021 edition

---

## App Factory Pattern

Use a function to build the `App` (avoids circular dependencies, easy to test):

```rust
// src/app.rs
use actix_web::{web, App};

pub fn create_app() -> App<...> {
    App::new()
        .service(web::scope("/v1").configure(routes::configure))
}

// src/main.rs
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| create_app())
        .bind("0.0.0.0:8080")?
        .run()
        .await
}
```

---

## Router Structure

Organize routes by domain using `web::scope` and `ServiceConfig`:

```rust
// src/routes/backtests.rs
use actix_web::{web, HttpResponse};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/backtests")
            .route("", web::get().to(list_backtests))
            .route("/{id}", web::get().to(get_backtest)),
    );
}

async fn get_backtest(path: web::Path<u64>) -> HttpResponse {
    let id = path.into_inner();
    HttpResponse::Ok().json(serde_json::json!({ "id": id }))
}
```

---

## Dependency Injection (App Data)

Use `web::Data<T>` for shared resources (DB pools, config):

```rust
use actix_web::web;
use std::sync::Arc;

// Register at startup
let db_pool = Arc::new(create_pool());
App::new()
    .app_data(web::Data::new(db_pool))
    .configure(routes::configure)

// In handler
async fn my_handler(pool: web::Data<Arc<DbPool>>) -> HttpResponse {
    // use pool
    HttpResponse::Ok().finish()
}
```

---

## Request / Response with Serde

Use `serde::{Serialize, Deserialize}` for JSON bodies:

```rust
use serde::{Deserialize, Serialize};
use actix_web::{web, HttpResponse};

#[derive(Deserialize)]
struct CreateBacktest {
    name: String,
    start_date: String,
}

#[derive(Serialize)]
struct BacktestResponse {
    id: u64,
    name: String,
}

async fn create_backtest(body: web::Json<CreateBacktest>) -> HttpResponse {
    let response = BacktestResponse { id: 1, name: body.name.clone() };
    HttpResponse::Created().json(response)
}
```

---

## Error Handling

Implement `ResponseError` for domain errors:

```rust
use actix_web::{HttpResponse, ResponseError};
use std::fmt;

#[derive(Debug)]
pub enum AppError {
    NotFound(String),
    Internal(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AppError::NotFound(msg) => write!(f, "Not found: {}", msg),
            AppError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        match self {
            AppError::NotFound(_) => HttpResponse::NotFound().json(serde_json::json!({"error": self.to_string()})),
            AppError::Internal(_) => HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"})),
        }
    }
}
```

---

## Startup / Shutdown

Use `HttpServer` lifecycle hooks:

```rust
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // startup: init resources here
    let pool = create_pool().await;

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .configure(routes::configure)
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
    // shutdown: resources dropped here
}
```

---

## Related Context

- **Testing** → `testing.md`
- **Cargo commands** → `cargo.md`
- **API Design Principles** → `../../principles/api-design.md`
