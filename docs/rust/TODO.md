# Rust Documentation Improvements

Review status (after uncommitted-change review):

- The architecture-book consolidation is structurally consistent and the new
  mdBook layout (`docs/rust/book/`) is a clear improvement over the deleted
  standalone guides.
- The deleted `docs/rust/ARCHITECTURE.md`, `docs/rust/technology-definition.md`,
  and `docs/project-structure-and-best-practices/` content has been retired or
  absorbed into the book, `MIGRATION.md`, `CONTRIBUTING.md`, and
  `docs/rust/RETIREMENT.md`.
- Module specifications from `docs/classes-definitions/` now live under
  `docs/rust/book/specs/` and are part of the mdBook.
- Source changes include module-level and public-API rustdoc for `node`,
  `plan`, `scene`, `text`, and `render`.
- `cargo fmt --check`, `cargo test --workspace`, `cargo doc --no-deps`, and
  `docs/rust/scripts/check-book.sh` all pass locally.

## High priority

- [x] Make the mdBook build reproducible. Pin or document the required `mdbook`
  version and make `mdbook build docs/rust` run in the project validation flow.
  - [x] Added `docs/rust/book-output/` to the root `.gitignore`.
  - [x] Replaced the empty `git-repository-url` in `docs/rust/book.toml` with a
        commented placeholder.
  - [x] Added `docs/rust/.mdbook-version` pinning `mdbook` to `0.4.40`.
  - [x] Added `docs/rust/scripts/build-book.sh` and
        `docs/rust/scripts/check-book.sh` to install the pinned version and run
        the build + link check.
- [x] Add continuous validation for the Rust documentation. Run the mdBook
  build, local link checks, and `cargo test --workspace --doc` where applicable.
  - [x] Added `.github/workflows/docs.yml` to validate docs and Rust checks on
        changes to `docs/rust/**`, `.github/workflows/docs.yml`, and `README.md`.
  - [x] Note: this package is a binary crate, so `cargo test --doc` does not
        apply. Doctests will become runnable only if a library target is added.
- [x] Create an ADR index and template for the open decisions in
  `book/architecture/technology.md`: Vulkan crate and ownership, platform
  lifecycle, ECS versus explicit systems, physics/audio/persistence strategy,
  telemetry, and networking.
- [x] Add module-level and public-API rustdoc to the existing source crates
  (`node`, `plan`, `scene`, `text`, `render`). Every public type and function
  states its contract, errors, ownership, and an example where the API is not
  self-evident.
- [x] Record a content-retirement map: for each deleted guide page, note which
  book chapter or file now owns its guidance, and verify that no unique
  decisions were dropped. See `docs/rust/RETIREMENT.md`.
- [x] Move `docs/classes-definitions/` into the `docs/rust/` handbook.
  - [x] Created `docs/rust/book/specs/` and moved the module specs.
  - [x] Created `docs/rust/book/specs/index.md`.
  - [x] Added a `# Specifications` part to `docs/rust/book/SUMMARY.md`.
  - [x] Retired `docs/classes-definitions/ARCHITECTURE.md` after confirming the
        target-architecture chapters supersede it.
  - [x] Updated `README.md`, source module comments, and internal links.
  - [x] Deleted the old `docs/classes-definitions/` directory.

## Medium priority

- [ ] Add the documentation link checker and required-section checks described
  by `docs/rust/scripts/README.md`.
  - Partial: a Markdown link checker exists in `docs/rust/scripts/check-links.py`.
    Required-section checks are not yet implemented.
- [x] Link `REVIEW_CHECKLIST.md` from `CONTRIBUTING.md` and require reviewers to
  use it for architecture and practice changes.
- [ ] Add the first headless, compile-tested examples listed in
  `docs/rust/examples/README.md`, then link each example to the architecture
  rule it demonstrates.
- [ ] Define how GPU and platform-specific documentation tests are separated
  from headless documentation checks in CI.
- [x] Add a short documentation index to the root `README.md` that points to the
  Rust book, migration plan, contribution guidelines, style guide, and review
  checklist.

## Low priority

- [ ] Review external links in `book/references/curated-links.md` quarterly and
  record changes to sources used by architecture decisions.
- [ ] After each migration stage, update `MIGRATION.md` with the completed
  baseline, acceptance evidence, and the next rollback point.
- [ ] Add doctests for the geometry/topology invariants already covered by
  unit tests (`node`, `plan`, `scene`) so the public API examples are also
  exercised by `cargo test --doc`.
  - Blocked: doctests require a library target; the current package is a binary
    crate.

## Validation commands

```text
cargo fmt --check
cargo test --workspace
cargo doc --no-deps
docs/rust/scripts/check-book.sh
```

For CI linting, also run:

```text
cargo clippy --workspace --all-targets --all-features -- -D warnings
```
