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

`docs/rust/scripts/check-sections.py` enforces the minimum page structure by
category: every guidance page needs a summary (or overview) and a key-points
(or rules/use/avoid) section; pattern pages also need an example; decision
records need context, decision, and consequences. Beyond that minimum, pages
should add pitfalls, tests, and authoritative references where they help.
Examples must be small enough to compile and review.

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
