# Crate Boundaries

Each crate has one reason to change and an explicit public surface.

## Engine

Owns reusable geometry, topology, renderer-neutral scene data, rendering
backend integration, resource ownership, and platform abstractions. It exposes
capabilities and data contracts, not backend handles or filesystem policy.

## Game

Owns startup, game state, world rules, input mapping, UI policy, and content.
It consumes engine APIs and should remain testable without opening a window when
logic does not require a renderer.

## Tools

Owns asset inspection, preprocessing, validation, and developer commands. Tool
outputs are deterministic and versioned when they become runtime inputs.

## Boundary review

Before exposing a type across crates, ask:

1. Is this a domain contract or an implementation detail?
2. Who owns the data and who may mutate it?
3. Can the type be tested without the platform or GPU?
4. Does the dependency point toward the engine foundation?
