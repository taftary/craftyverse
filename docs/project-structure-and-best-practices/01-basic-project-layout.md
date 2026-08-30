# Basic Project Layout

Cargo handles project creation, building, testing, and dependency management. A new project created with `cargo new my_project` gets a clean, standardized layout out of the box:

```
my_project/
├── Cargo.toml
├── Cargo.lock
└── src/
    └── main.rs
```

## Cargo.toml

The heart of the project's configuration. It defines:

- Project metadata (name, version, authors).
- Dependencies (external crates to use).
- Build settings and features.

Example:

```toml
[package]
name = "my_project"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = "1.0"
```

## Cargo.lock

Automatically generated after the first build. It records the exact versions of dependencies used, ensuring reproducible builds.

- **Applications**: commit `Cargo.lock` to version control.
- **Libraries**: usually do not commit it, so consumers can resolve dependencies themselves.

## src/main.rs

The default entry point of a binary project:

```rust
fn main() {
    println!("Hello, world!");
}
```

As the project grows, move most of the logic into separate modules or a `lib.rs` file, leaving `main.rs` as a thin entry point.
