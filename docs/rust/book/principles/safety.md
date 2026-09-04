# Safety Boundaries

## Summary

Safe Rust is the default. Unsafe code is permitted only for measured
capabilities that safe Rust cannot provide, such as platform or GPU boundaries.

## Key points

- Every unsafe block must have a small scope and a documented safety invariant.
- Validate invariants at the safe API boundary and test the invariant and failure cases.
- Assign an owner responsible for reviewing unsafe changes.
- FFI and backend handles belong in engine modules; game and tools consume safe wrappers.

Every unsafe block must have:

- a small scope;
- a documented safety invariant;
- validation at the safe API boundary;
- tests for the invariant and failure cases;
- an owner responsible for reviewing changes.

FFI and backend handles belong in engine modules. Game and tools code consume
safe wrappers and must not reproduce unsafe assumptions.

**References**

[Rustonomicon](https://doc.rust-lang.org/nomicon/)
