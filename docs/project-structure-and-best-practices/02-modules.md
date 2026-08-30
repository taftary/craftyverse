# Splitting Code into Modules

Modules are the primary way to organize Rust code into smaller, manageable pieces. Instead of cramming everything into `main.rs`, group related functions, structs, and enums into separate files or directories.

## Declaring a Module

Instead of keeping helper functions in `main.rs`:

```rust
fn main() {
    let result = add(2, 3);
    println!("Result: {}", result);
}

fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

Move them into a new file `src/utils.rs`:

```rust
// src/utils.rs
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

Then declare the module in `main.rs` and use it:

```rust
mod utils;

fn main() {
    let result = utils::add(2, 3);
    println!("Result: {}", result);
}
```

- `mod utils;` tells Rust to look for a file named `utils.rs` in the `src/` directory.
- `pub` makes the function visible outside of the `utils` module.

## Organizing with Submodules

For nested structures, create a folder with a `mod.rs`:

```
src/
├── main.rs
└── models/
    ├── mod.rs
    └── user.rs
```

`src/models/user.rs`:

```rust
pub struct User {
    pub id: u32,
    pub name: String,
}
```

`src/models/mod.rs`:

```rust
pub mod user;
```

`src/main.rs`:

```rust
mod models;

fn main() {
    let user = models::user::User {
        id: 1,
        name: String::from("Alice"),
    };

    println!("User: {} with ID {}", user.name, user.id);
}
```

## Best Practices for Modules

- Use one file per module when possible (`utils.rs`, `config.rs`, etc.).
- Use directories with `mod.rs` when grouping related modules (`models/mod.rs` with `models/user.rs`, `models/product.rs`).
- Use `pub(crate)` to expose items only within the crate instead of making everything `pub`.
