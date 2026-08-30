# Dependency Management

Without discipline, a project can quickly suffer from dependency bloat, version conflicts, and longer compile times. Managing dependencies well keeps the project clean and efficient.

## 1. Keep Cargo.toml Clean

List only the dependencies actually needed.

Bad (bloated, unused crates):

```toml
[dependencies]
serde = "1.0"
tokio = "1.40"
rand = "0.9"
regex = "1.11"
chrono = "0.4"
```

Better (minimal, focused):

```toml
[dependencies]
serde = "1.0"
tokio = { version = "1.40", features = ["full"] }
```

Always remove unused crates to speed up compile times and reduce binary size.

## 2. Use Features to Reduce Bloat

Many crates have optional features. Enable only what is needed:

```toml
serde = { version = "1.0", features = ["derive"] }
```

This includes `serde_derive` for serialization but avoids pulling in unnecessary extras.

## 3. Manage Versions

Cargo uses semantic versioning. Common patterns:

- `serde = "1.0"` → any compatible `1.x.y` version.
- `serde = "1.0.188"` → exact version.
- `serde = ">=1.0.100, <2.0.0"` → custom range.

- For **libraries**, keep versions flexible to maximize compatibility.
- For **applications**, pin exact versions for reproducibility (via `Cargo.lock`).

## 4. Workspaces for Multi-Crate Projects

When a project grows into multiple components (e.g. CLI + library + API), use a Cargo workspace:

```
my_project/
├── Cargo.toml
├── cli/
│   └── Cargo.toml
├── core/
│   └── Cargo.toml
└── api/
    └── Cargo.toml
```

Top-level `Cargo.toml`:

```toml
[workspace]
members = ["cli", "core", "api"]
```

Benefits:

- Shared `Cargo.lock` → consistent versions across crates.
- `cargo build` → builds everything in one go.
- Logical separation of responsibilities (core logic vs. API vs. CLI).

## 5. Security and Auditing

Check for known vulnerabilities in dependencies:

```
cargo install cargo-audit
cargo audit
```

With a clean `Cargo.toml`, minimal features, and workspaces, the project stays lean, maintainable, and scalable.
