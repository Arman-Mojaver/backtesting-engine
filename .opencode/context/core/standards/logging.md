<!-- Context: standards/logging | Priority: critical | Version: 2.0 | Updated: 2026-05-23 -->
# Logging Standard

## Quick Reference

**Golden Rule**: Log at the edges. Productive code stays silent.

**One lever**: `RUST_LOG` env var. `info` → INFO. `debug` → DEBUG everywhere.

**Where logging lives** (the only three surfaces):
1. **API edge** — request/response middleware + error handlers
2. **DB edge** — query execution events
3. **Domain warnings** — only for self-healed conditions (e.g. div-by-zero skip)

**Never log**:
- Function arguments (PII, payload size)
- SQL bind parameters
- Request/response bodies
- Exception messages in HTTP responses (use `request_id` for tracing)

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  HTTP edge — middleware/                                     │
│  ├─ request logger: method/path/status/duration_ms/req_id   │  INFO
│  └─ error handlers: 500 + ERROR / 4xx + WARN                │  ERROR/WARN
├─────────────────────────────────────────────────────────────┤
│  Use-case — src/use_cases/                                   │
│  └─ log::debug! on execute() entry/exit + duration          │  DEBUG
├─────────────────────────────────────────────────────────────┤
│  Domain — src/domain/                                        │
│  └─ WARN only for self-healed conditions                    │  WARN
│     (return Err for everything else; edge logs them)        │
├─────────────────────────────────────────────────────────────┤
│  DB edge — src/adapters/db.rs                                │
│  └─ per-query DEBUG, slow-query WARN                        │  DEBUG/WARN
└─────────────────────────────────────────────────────────────┘
```

## Levels

| Level | Use for | Example |
|---|---|---|
| `DEBUG` | Function entry/exit, SQL execution, intermediate state | use-case timing, query events |
| `INFO` | Request received/completed, app lifecycle | HTTP middleware request line |
| `WARN` | Recoverable: slow query, business rule self-heal, 4xx errors | slow query (>500ms), domain self-heal |
| `ERROR` | Unhandled error, 5xx | unhandled error handler |

## Configuration

**One env var**: `RUST_LOG` (e.g. `info`, `debug`, `backtesting_engine=debug`).

| Env | `RUST_LOG` | Root level |
|---|---|---|
| local / dev / prod / staging | `info` | INFO |
| any | `debug` | DEBUG |
| testing | `warn` | WARN (keeps test output quiet) |
| testing | `debug` | DEBUG (debug a failing test) |

Hard-coded silencers (independent of `RUST_LOG`):
- `sqlx` or DB driver → WARN (project's own query events handle the rest)

## Logger Naming

Always use the `log` crate macros with the module path:

```rust
use log::{debug, info, warn, error};

// Produces: backtesting_engine::use_cases::run_backtest
fn run_backtest(...) {
    debug!("starting backtest execution");
    // ...
    info!("backtest complete");
}
```

## Decorators / Instrumentation

Use a timing wrapper for use-cases:

```rust
use std::time::Instant;
use log::debug;

pub fn timed<F, T>(name: &str, f: F) -> T
where
    F: FnOnce() -> T,
{
    let start = Instant::now();
    let result = f();
    debug!("{} completed in {}ms", name, start.elapsed().as_millis());
    result
}

// Usage
let result = timed("run_backtest", || backtest_service.execute(params));
```

## What Goes Where

### HTTP edge (`src/middleware/`)
- **Request logger** logs each request once at INFO with structured fields:
  `method`, `path`, `status`, `duration_ms`. Sets `request_id` so every log record in that request inherits it.
- **Error handler** catches unhandled errors, logs ERROR + stack trace, returns 500 with
  `{"error": "Internal server error", "request_id": "..."}`. Never leaks error details.
- **4xx handler** logs WARN with `error_type`. Body returns `{"error": "<message>"}`.

### DB edge (`src/adapters/db.rs`)
- Wrap query execution to emit DEBUG `"query executed"` with `duration_ms`.
- Queries over threshold additionally emit WARN `"slow query"`.
- **Bind parameters are never logged.**

### Use-cases (`src/use_cases/`)
- `timed(...)` wrapper on `execute()` only.
- No `match`/`if let` solely for logging — let errors propagate to the global handler.

### Domain (`src/domain/`)
- **Errors return `Err(...)`**. Logging happens at the HTTP handler.
- **Warnings log inline** — only when the domain self-heals a condition:

  ```rust
  use log::warn;

  if denominator == 0.0 {
      warn!("skipped item: would divide by zero; id={}", item.id);
      continue;
  }
  ```

  Don't log INFO/DEBUG inside domain — that's noise for happy-path operations.

## Adding a New Domain Error

1. Define the error variant in `src/domain/errors.rs`.
2. Implement `ResponseError` for it in `src/adapters/error_handlers.rs`.
3. Register the handler in `src/app.rs` if needed.

## Anti-Patterns

❌ Logging inside repository methods (use DB edge events)
❌ Logging inside calculators except for self-healed warnings
❌ `match err { ... log and re-raise }` inside use-cases (handler does it)
❌ Logging arguments, SQL params, or request bodies
❌ Returning error details in 500 responses
❌ `log::logger()` global access (use macros: `debug!`, `info!`, `warn!`, `error!`)
❌ Per-environment `LOG_LEVEL` config (use `RUST_LOG` env var)

## Testing

Use `env_logger` or `test-log` crate to capture log output in tests. No mocking of internals.

```rust
#[cfg(test)]
mod tests {

    async fn test_request_is_logged() {
        // Use test::init_service and assert on response status
        // Log capture via RUST_LOG=debug in test environment
    }
}
```
