# Clean Code Best Practices

Writing working code is only half the battle — writing clean, maintainable code is what keeps a Rust project sustainable as it grows.

## 1. Follow Rust Naming Conventions

- **Modules & files** → `snake_case` (e.g. `utils.rs`, `user_profile.rs`).
- **Functions & variables** → `snake_case` (e.g. `calculate_total`, `user_id`).
- **Structs, Enums, Traits** → `PascalCase` (e.g. `User`, `OrderStatus`, `Drawable`).
- **Constants & Statics** → `SCREAMING_SNAKE_CASE` (e.g. `MAX_CONNECTIONS`).

```rust
struct UserProfile {
    user_id: u32,
    user_name: String,
}
```

## 2. Keep Modules Small and Focused

Each module should have a single responsibility. Instead of a huge `utils.rs` that does everything, break it down:

```
src/
├── utils/
│   ├── mod.rs
│   ├── math.rs
│   └── string.rs
```

```rust
// utils/mod.rs
pub mod math;
pub mod string;
```

## 3. Use Visibility Wisely (pub, pub(crate))

Everything in Rust is private by default. Use visibility modifiers carefully:

- `pub` → public to the world.
- `pub(crate)` → accessible only within the current crate (good for internal APIs).
- `pub(super)` → accessible only to the parent module.

```rust
pub(crate) fn internal_helper() {
    println!("Only available within this crate");
}
```

This prevents exposing unnecessary internal details.

## 4. Keep Functions Short and Focused

Functions should do one thing well. If a function grows too large, break it into smaller helpers.

Bad (hard to read, multi-purpose):

```rust
fn process_user(id: u32, name: &str) {
    println!("Creating user: {}", name);
    // validation
    if name.is_empty() {
        panic!("Name cannot be empty");
    }
    // more logic...
}
```

Better (cleaner, testable):

```rust
fn validate_name(name: &str) {
    if name.is_empty() {
        panic!("Name cannot be empty");
    }
}

fn process_user(id: u32, name: &str) {
    println!("Creating user: {}", name);
    validate_name(name);
    // other logic...
}
```

## 5. Separate Concerns

Don't mix business logic, configuration, and I/O in the same module. For example, in a web service:

- `models/` → structs and data types.
- `services/` → business logic.
- `handlers/` → web request handling.
- `config/` → configuration and environment setup.

This makes the project easier to test and extend.

## 6. Write Tests Alongside Your Code

Use unit tests directly inside the module:

```rust
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 3), 5);
    }
}
```

This keeps tests close to the implementation and helps maintain correctness.
