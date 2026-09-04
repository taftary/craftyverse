# Testing and Doctests

Tests protect crate contracts and migration boundaries.

- Unit tests cover local invariants beside the implementation.
- Integration tests cover public APIs across crate boundaries.
- Doctests validate examples users are expected to copy.
- Property tests cover geometry and topology invariants over generated inputs.
- Benchmarks support measured performance decisions; they do not define correctness.
- GPU and device tests are explicit suites separate from headless CI.

Every migrated responsibility keeps its existing tests until the new crate owns
the same contract. A file move is complete only when the test location and API
owner are clear.

```text
cargo test --workspace
cargo test --doc --workspace
```
