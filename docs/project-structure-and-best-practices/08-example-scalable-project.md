# Example: Building a Small, Scalable Project

A simple CLI Todo app showing how all the principles fit together:

- `models/` → defines the `Todo` struct.
- `services/` → manages business logic.
- `cli/` → handles command-line arguments.
- `main.rs` → a thin entry point.

## Step 1: Project Layout

```
todo_app/
├── Cargo.toml
└── src/
    ├── main.rs
    ├── lib.rs
    ├── models/
    │   ├── mod.rs
    │   └── todo.rs
    ├── services/
    │   ├── mod.rs
    │   └── todo_service.rs
    └── cli.rs
```

## Step 2: Define the Model

`src/models/todo.rs`:

```rust
#[derive(Debug)]
pub struct Todo {
    pub id: u32,
    pub title: String,
    pub completed: bool,
}
```

`src/models/mod.rs`:

```rust
pub mod todo;
```

## Step 3: Create a Service Layer

`src/services/todo_service.rs`:

```rust
use crate::models::todo::Todo;

pub struct TodoService {
    todos: Vec<Todo>,
}

impl TodoService {
    pub fn new() -> Self {
        Self { todos: Vec::new() }
    }

    pub fn add(&mut self, title: &str) {
        let id = (self.todos.len() + 1) as u32;
        let todo = Todo {
            id,
            title: title.to_string(),
            completed: false,
        };
        self.todos.push(todo);
    }

    pub fn list(&self) -> &Vec<Todo> {
        &self.todos
    }

    pub fn complete(&mut self, id: u32) {
        if let Some(todo) = self.todos.iter_mut().find(|t| t.id == id) {
            todo.completed = true;
        }
    }
}
```

`src/services/mod.rs`:

```rust
pub mod todo_service;
```

## Step 4: Handle CLI Input

Kept simple using `std::env`. `src/cli.rs`:

```rust
use std::env;

pub enum Command {
    Add(String),
    List,
    Complete(u32),
    Unknown,
}

pub fn parse_args() -> Command {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        return Command::Unknown;
    }

    match args[1].as_str() {
        "add" if args.len() > 2 => Command::Add(args[2..].join(" ")),
        "list" => Command::List,
        "complete" if args.len() == 3 => {
            if let Ok(id) = args[2].parse() {
                Command::Complete(id)
            } else {
                Command::Unknown
            }
        }
        _ => Command::Unknown,
    }
}
```

## Step 5: The Library Wrapper

`src/lib.rs`:

```rust
pub mod models;
pub mod services;
pub mod cli;
```

## Step 6: Main Entry Point

`src/main.rs`:

```rust
use todo_app::{cli, services::todo_service::TodoService};

fn main() {
    let command = cli::parse_args();
    let mut service = TodoService::new();

    match command {
        cli::Command::Add(task) => {
            service.add(&task);
            println!("Added: {}", task);
        }
        cli::Command::List => {
            for todo in service.list() {
                println!("[{}] {} - {}",
                    todo.id,
                    todo.title,
                    if todo.completed { "Done" } else { "Pending" }
                );
            }
        }
        cli::Command::Complete(id) => {
            service.complete(id);
            println!("Marked task {} as complete", id);
        }
        cli::Command::Unknown => {
            println!("Usage:");
            println!("  todo_app add <task>");
            println!("  todo_app list");
            println!("  todo_app complete <id>");
        }
    }
}
```

## Step 7: Running the App

```
cargo build
cargo run -- add "Learn Rust"
cargo run -- add "Build a CLI app"
cargo run -- list
cargo run -- complete 1
cargo run -- list
```

Output:

```
Added: Learn Rust
Added: Build a CLI app
[1] Learn Rust - Pending
[2] Build a CLI app - Pending
Marked task 1 as complete
[1] Learn Rust - Done
[2] Build a CLI app - Pending
```

With this structure, business logic, models, and CLI handling are neatly separated — making it easier to scale later (e.g. adding persistence, logging, or API support).
