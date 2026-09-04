# Rust Documentation Improvements

Review status (after uncommitted-change review):

- The architecture-book consolidation is structurally consistent and the new
  mdBook layout (`docs/rust/book/`) is a clear improvement over the deleted
  standalone guides.
- The deleted `docs/rust/ARCHITECTURE.md`, `docs/rust/technology-definition.md`,
  and `docs/project-structure-and-best-practices/` content has been retired or
  absorbed into the book, `CONTRIBUTING.md`, and `docs/rust/RETIREMENT.md`.
- Module specifications from `docs/classes-definitions/` now live under
  `docs/rust/book/specs/` and are part of the mdBook.
- Source changes include module-level and public-API rustdoc for `node`,
  `plan`, `scene`, `text`, and `render`.
- `cargo fmt --check`, `cargo test --workspace --all-targets`,
  `cargo test --doc --workspace`, `cargo doc --no-deps`, and
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
  - [x] Note: the game crate is a binary crate, but `planet-crafter-engine` is
        a library crate, so `cargo test --doc --workspace` runs doctests on the
        engine public API.
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

- [x] Add the documentation link checker and required-section checks described
  by `docs/rust/scripts/README.md`.
  - [x] `docs/rust/scripts/check-links.py` checks internal Markdown links.
  - [x] `docs/rust/scripts/check-sections.py` checks required sections per page
        category (architecture, principles, patterns, practices, specs, decisions,
        examples, references).
- [x] Link `REVIEW_CHECKLIST.md` from `CONTRIBUTING.md` and require reviewers to
  use it for architecture and practice changes.
- [x] Add the first headless, compile-tested examples listed in
  `docs/rust/examples/README.md`, then link each example to the architecture
  rule it demonstrates.
  - [x] Created `docs/rust/examples/` workspace crate with `engine_api`,
        `validated_resource`, `state_transitions`, `message_flow`, and
        `renderer_neutral_scene` examples.
  - [x] Each example links to the relevant architecture rule in its module docs.
  - [x] Examples are compile-tested through `cargo test --workspace --examples`.
- [x] Define how GPU and platform-specific documentation tests are separated
  from headless documentation checks in CI.
  - [x] Added a `gpu` feature and `viewer` example gated with
        `required-features` in `docs/rust/examples/Cargo.toml`.
  - [x] Documented the separation in `docs/rust/book/practices/testing.md`.
  - [x] Split `.github/workflows/docs.yml` into `headless-docs` and
        `gpu-and-platform` jobs.
- [x] Add a short documentation index to the root `README.md` that points to the
  Rust book, contribution guidelines, style guide, and review checklist.

## Low priority

- [x] Review external links in `book/references/curated-links.md` quarterly and
  record changes to sources used by architecture decisions.
  - [x] Checked all links on 2026-09-04; `w3reference.com` blocks plain HEAD
        requests but serves the page with a browser user agent.
- [x] Add doctests for the geometry/topology invariants already covered by
  unit tests (`node`, `plan`, `scene`) so the public API examples are also
  exercised by `cargo test --doc`.
  - [x] Added geometry-invariant doctests to `Node::new`.
  - [x] Added subdivision-invariant doctests to `Node::split`.
  - [x] Added mesh-saturation doctests to `Plan::generate`.
  - [x] Added clip-space doctests to `scene::build_scene`.
  - Note: doctests are valid because `planet-crafter-engine` is a library crate.
    The game crate remains a binary crate with no doctests.

## Validation commands

```text
cargo fmt --check
cargo test --workspace --all-targets
cargo doc --no-deps
docs/rust/scripts/check-book.sh
```

For CI linting, also run:

```text
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

The `gpu` example is excluded from the headless command because it requires a
Vulkan-capable display. Compile-check it separately with:

```text
cargo check --example viewer -p planet-crafter-examples --features gpu
```
