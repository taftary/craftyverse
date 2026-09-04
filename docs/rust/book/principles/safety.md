# Safety Boundaries

Safe Rust is the default. Unsafe code is permitted only when it provides a
measured capability that safe Rust cannot provide, such as a platform or GPU
boundary.

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
