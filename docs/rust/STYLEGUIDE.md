# Rust Documentation Style Guide

## Status language

Use `Current baseline`, `Target`, `Planned`, and `Open`. Never describe a
planned crate, dependency, platform, or workflow as implemented.

## Rust conventions

- Use Rust terminology: crate, module, type, trait, ownership, and borrowing.
- Prefer `snake_case` for functions and modules and `PascalCase` for types.
- Keep public APIs documented with contracts, errors, ownership, and examples.
- Prefer small, named types over primitive values with implicit units.
- Use ASCII in documentation unless a technical notation requires otherwise.

## Page structure

Guidance pages should contain a summary, use/avoid guidance, key points, a
minimal example, pitfalls, tests, and authoritative references. Examples must be
small enough to compile and review.

## Validation commands

`cargo fmt --check` validates Rust source files managed by Cargo. It does not
format Markdown under `docs/`. Validate the documentation separately with the
mdBook build and a local Markdown link check:

```text
cargo fmt --check
docs/rust/scripts/check-book.sh
```

Documentation examples that are part of a Cargo package must still pass the
normal Rust formatting, test, and Clippy checks.

## Architecture language

The Rust book is the target blueprint. Current implementation facts belong in
source documentation and architecture decision records, not in the target
blueprint.
