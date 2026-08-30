# Rust Project Structure & Best Practices

Guidelines for keeping this project's Rust code clean, scalable, and maintainable.

Inspired by [Rust Project Structure and Best Practices for Clean, Scalable Code](https://www.djamware.com/post/rust-project-structure-and-best-practices-for-clean-scalable-code) (Djamware).

## Instructions

1. [Basic Project Layout](01-basic-project-layout.md) — Cargo conventions: `Cargo.toml`, `Cargo.lock`, `src/main.rs`.
2. [Splitting Code into Modules](02-modules.md) — modules, files, and directories with `mod.rs`.
3. [Library vs Binary Projects](03-library-vs-binary.md) — when to use `main.rs`, `lib.rs`, or both.
4. [Organizing with Folders](04-folder-organization.md) — `src/`, `tests/`, `examples/`, `benches/`, and optional folders.
5. [Clean Code Best Practices](05-clean-code-practices.md) — naming, small modules, visibility, short functions, separation of concerns, tests.
6. [Error Handling & Logging](06-error-handling-and-logging.md) — centralized error types, `anyhow`, `log` / `tracing`.
7. [Dependency Management](07-dependency-management.md) — clean `Cargo.toml`, features, versioning, workspaces, auditing.
8. [Example: Small Scalable Project](08-example-scalable-project.md) — a Todo CLI applying all the principles together.

## Key Takeaways

- Start simple, scale when needed — don't over-engineer, but refactor as the codebase grows.
- Use modules and folders to separate concerns and keep the codebase navigable.
- Leverage workspaces when the project grows into multiple crates (CLI, library, API, etc.).
- Keep dependencies minimal and well-managed to avoid bloat, long compile times, and security issues.
- Follow Rust idioms and conventions (naming, module usage, `Cargo.toml` hygiene) for consistency and readability.
