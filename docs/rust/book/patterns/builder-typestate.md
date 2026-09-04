# Builder and Typestate

## Summary

Use a builder when construction has many optional values or validation rules.
Use typestate when a small, stable state transition must be enforced at compile
time.

## Key points

- Builders improve call-site readability but add API surface.
- Typestate prevents invalid transitions but can increase the number of types.
- Prefer a validated constructor when neither benefit is needed.
- Test both successful construction and rejected invariants.

## Example

```rust
#[derive(Debug, PartialEq)]
pub struct RenderConfig { width: u32, height: u32 }

pub struct RenderConfigBuilder { width: u32, height: u32 }

impl RenderConfigBuilder {
    pub fn new() -> Self { Self { width: 1, height: 1 } }
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }
    pub fn build(self) -> Result<RenderConfig, &'static str> {
        (self.width > 0 && self.height > 0)
            .then_some(RenderConfig { width: self.width, height: self.height })
            .ok_or("render size must be positive")
    }
}
```

## Trade-offs

Builders improve call-site readability but add API surface. Typestate prevents
invalid transitions but can increase the number of types. Prefer a validated
constructor when neither benefit is needed.

```rust
#[derive(Debug, PartialEq)]
pub struct RenderConfig { width: u32, height: u32 }

pub struct RenderConfigBuilder { width: u32, height: u32 }

impl RenderConfigBuilder {
    pub fn new() -> Self { Self { width: 1, height: 1 } }
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }
    pub fn build(self) -> Result<RenderConfig, &'static str> {
        (self.width > 0 && self.height > 0)
            .then_some(RenderConfig { width: self.width, height: self.height })
            .ok_or("render size must be positive")
    }
}
```

Test both successful construction and rejected invariants.
