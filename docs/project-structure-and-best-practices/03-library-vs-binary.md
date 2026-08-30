# Library vs Binary Projects

Cargo defaults to a binary project — it compiles into an executable with a `main.rs` entry point. Rust also supports library projects, which produce reusable crates. Knowing when to use `main.rs`, `lib.rs`, or both is key to building scalable applications.

## Binary Projects (main.rs)

A binary project has an entry point function:

```rust
// src/main.rs
fn main() {
    println!("Hello, world!");
}
```

Suitable for command-line tools, apps, or services that run as standalone executables.

## Library Projects (lib.rs)

A library has no `main` function. It exposes functions, structs, and modules that other projects (or binaries) can reuse:

```rust
// src/lib.rs
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}
```

## Combining main.rs and lib.rs

In larger applications, keep business logic inside `lib.rs` and leave `main.rs` as a thin wrapper:

```
src/
├── lib.rs
└── main.rs
```

`src/lib.rs`:

```rust
pub fn run_app() {
    println!("App is running!");
}
```

`src/main.rs`:

```rust
use my_project::run_app;

fn main() {
    run_app();
}
```

Advantages:

- Core logic can be reused in multiple binaries.
- Code is easier to test (unit tests against the library).
- `main.rs` stays clean and focused on orchestration, not business logic.

## When to Use Each

- **Only `main.rs`** → simple apps, scripts, prototypes.
- **Only `lib.rs`** → pure libraries or crates intended for reuse.
- **Both** → best practice for most real-world applications where separation of logic and entry points is wanted.
