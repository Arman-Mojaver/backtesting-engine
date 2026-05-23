<!-- Context: development/data/db-patterns | Priority: high | Version: 2.0 | Updated: 2026-05-23 -->

# Data Layer Patterns

**Stack**: Rust 2021 edition · actix-web 4.x

> ⚠️ This project does not currently have a database dependency in `Cargo.toml`.
> Add patterns here when a DB crate is introduced (e.g. `sqlx`, `diesel`, `sea-orm`).

---

## Recommended Crates

| Crate | Use case |
|-------|----------|
| `sqlx` | Async SQL with compile-time query checking |
| `diesel` | Sync ORM with strong type safety |
| `sea-orm` | Async ORM built on sqlx |

---

## Repository Pattern (Trait-based)

Define a trait in the domain layer; implement it in the adapters layer:

```rust
// src/domain/repositories.rs
pub trait PriceRepository: Send + Sync {
    async fn load_prices(&self, symbol: &str) -> Result<Vec<PricePoint>, AppError>;
}

// src/adapters/db_price_repository.rs
pub struct DbPriceRepository {
    pool: sqlx::PgPool,
}

impl PriceRepository for DbPriceRepository {
    async fn load_prices(&self, symbol: &str) -> Result<Vec<PricePoint>, AppError> {
        let rows = sqlx::query_as!(
            PricePoint,
            "SELECT date, close FROM prices WHERE symbol = $1 ORDER BY date",
            symbol
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}
```

---

## Connection Pool (sqlx example)

```rust
use sqlx::PgPool;

pub async fn create_pool(database_url: &str) -> PgPool {
    PgPool::connect(database_url)
        .await
        .expect("failed to connect to database")
}

// Register with actix-web
App::new()
    .app_data(web::Data::new(pool))
```

---

## Related Context

- **Actix-web patterns** → `../backend/rust/actix-patterns.md`
- **Backend navigation** → `../backend/navigation.md`
