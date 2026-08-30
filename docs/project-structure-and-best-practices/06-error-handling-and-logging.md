# Error Handling & Logging Structure

Rust's type system makes error handling explicit, which helps prevent runtime surprises. Design a structured error-handling system and combine it with logging for better observability.

## 1. Centralized Error Types

Instead of returning `Result<T, String>`, define a custom error type for the project using `thiserror`. This makes code more descriptive and easier to maintain.

```rust
// src/errors.rs
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MyAppError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Unknown error occurred")]
    Unknown,
}
```

Usage:

```rust
use crate::errors::MyAppError;

pub fn process_input(input: &str) -> Result<(), MyAppError> {
    if input.is_empty() {
        return Err(MyAppError::InvalidInput("Input cannot be empty".into()));
    }
    Ok(())
}
```

## 2. Simpler Error Handling with anyhow

For apps that don't need strict error typing, use `anyhow`. It provides an easy way to bubble up errors without defining enums everywhere:

```rust
use anyhow::Result;

fn run() -> Result<()> {
    let data = std::fs::read_to_string("config.json")?;
    println!("Config: {}", data);
    Ok(())
}
```

Great for prototyping or CLI tools.

## 3. Logging with log or tracing

### log + env_logger

```toml
[dependencies]
log = "0.4"
env_logger = "0.11"
```

```rust
use log::{info, warn, error};

fn main() {
    env_logger::init();

    info!("Application started");
    warn!("This is a warning");
    error!("Something went wrong");
}
```

Run with `RUST_LOG=info cargo run`.

### tracing (recommended for larger apps)

`tracing` provides structured, async-friendly logging with spans.

```toml
[dependencies]
tracing = "0.1"
tracing-subscriber = "0.3"
```

```rust
use tracing::{info, instrument};
use tracing_subscriber;

#[instrument]
fn calculate(x: i32, y: i32) -> i32 {
    x + y
}

fn main() {
    tracing_subscriber::fmt::init();

    let result = calculate(2, 3);
    info!("Calculation result: {}", result);
}
```

## 4. Project Structure for Errors & Logs

```
src/
├── main.rs
├── errors.rs
└── services/
    └── calculator.rs
```

- `errors.rs` → central place for custom error definitions.
- `services/` → business logic returning `Result<T, MyAppError>`.
- `main.rs` → initializes logger and handles top-level errors.

With centralized error handling and proper logging, Rust apps become more robust, debuggable, and production-ready.
