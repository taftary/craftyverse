# Testing and Doctests

## Summary

Tests protect crate contracts and migration boundaries. Separate headless tests
from GPU and device tests so CI stays fast and reliable.

## Key points

- Unit tests cover local invariants beside the implementation.
- Integration tests cover public APIs across crate boundaries.
- Doctests validate examples users are expected to copy.
- Property tests cover geometry and topology invariants over generated inputs.
- Benchmarks support measured performance decisions; they do not define correctness.
- GPU and device tests are explicit suites separate from headless CI.

Every migrated responsibility keeps its existing tests until the new crate owns
the same contract. A file move is complete only when the test location and API
owner are clear.

## Headless checks

The default local and CI commands compile and run every test and headless
example:

```text
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
cargo test --doc --workspace
```

`cargo test --workspace --all-targets` includes the example crate
(`docs/examples`) and compiles its example binaries, but skips examples
that require the `gpu` feature.

## GPU and platform-specific checks

Examples and tests that need a window, GPU, or device are gated behind the `gpu`
feature in `docs/examples`. The gate is declared with Cargo's
`required-features` so headless runs automatically skip them.

Compile-check the GPU example without running it:

```text
cargo check --example viewer -p planet-crafter-examples --features gpu
```

Run it only on a workstation with a Vulkan-capable display:

```text
cargo run --example viewer -p planet-crafter-examples --features gpu
```

CI keeps GPU checks in a separate job from headless documentation checks. The
headless job runs the commands above without `--features gpu`; the GPU job runs
on a runner that provides a display and Vulkan driver. Keeping the two jobs
separate prevents a missing GPU from blocking the handbook and Rust validation
pipeline.
