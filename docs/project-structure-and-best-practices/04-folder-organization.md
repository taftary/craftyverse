# Organizing with Folders

Cargo supports common folder conventions that keep code, tests, and examples separate and easy to maintain.

## src/ — Application Code

The main directory for source code:

- `main.rs` → entry point for binary projects.
- `lib.rs` → reusable library code.
- Submodules → additional `.rs` files or folders (`utils.rs`, `models/`, `services/`).

```
src/
├── main.rs
├── lib.rs
├── utils.rs
└── models/
    ├── mod.rs
    └── user.rs
```

## tests/ — Integration Tests

Rust supports two kinds of tests:

- **Unit tests** — placed inside the same file as the code, usually with `#[cfg(test)]`.
- **Integration tests** — placed in a separate `tests/` folder.

```
tests/
└── integration_test.rs
```

```rust
use my_project::greet;

#[test]
fn test_greet() {
    assert_eq!(greet("Alice"), "Hello, Alice!");
}
```

Run with `cargo test`.

## examples/ — Example Programs

Showcases how the library or app can be used. Each file in `examples/` compiles as its own binary:

```
examples/
└── hello.rs
```

```rust
use my_project::greet;

fn main() {
    println!("{}", greet("World"));
}
```

Run with `cargo run --example hello`.

## benches/ — Benchmarks

Benchmarking with the `criterion` crate:

```
cargo add criterion
```

```
benches/
└── my_benchmark.rs
```

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use my_project::greet;

fn bench_greet(c: &mut Criterion) {
    c.bench_function("greet", |b| b.iter(|| greet(black_box("Alice"))));
}

criterion_group!(benches, bench_greet);
criterion_main!(benches);
```

Run with `cargo bench`.

## Other Optional Folders

- `migrations/` → database migrations (when using ORMs like Diesel).
- `docs/` → extra documentation beyond Rustdoc.
- `assets/` → configs, images, or data files.

## Example Full Layout

```
my_project/
├── Cargo.toml
├── Cargo.lock
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── utils.rs
│   └── models/
│       ├── mod.rs
│       └── user.rs
├── tests/
│   └── integration_test.rs
├── examples/
│   └── hello.rs
├── benches/
│   └── my_benchmark.rs
└── assets/
    └── config.json
```

This structure keeps the project professional, modular, and ready for scaling.
