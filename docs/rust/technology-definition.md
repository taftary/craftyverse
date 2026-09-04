# Project Technology Definition (Rust Version)

## Overview

This project is a high-performance 3D mobile game built from scratch in Rust, using Vulkan as the sole graphics API. The goal is to maximize efficiency, safety, and performance on mobile GPUs while[...]

## Programming Language

**Rust**

- Provides memory safety without garbage collection, preventing common C++ issues like use-after-free and data races.
- Zero-cost abstractions allow high-performance systems programming comparable to C++.
- Strong ecosystem for tooling, testing, and modular architecture.

## Target Platforms & Build Tooling

- **Android**: Vulkan-capable devices (API 24+ as baseline; final minimum TBD based on device tier targets).
- **iOS**: Apple does not expose Vulkan natively — Vulkan support is provided through **MoltenVK** (Vulkan over Metal), keeping a single Vulkan codebase across both platforms.
- **Windowing & lifecycle**: `android-activity` crate for Android lifecycle/surface management; `winit` where a cross-platform abstraction is preferred.
- **Build tooling**:
  - `cargo-ndk` for Android builds and NDK integration.
  - Xcode project integration for iOS (Rust built as a static library, linked into an app target).
- **CI/CD**: automated `cargo build`, `cargo test`, `clippy`, and `rustfmt` checks on every change; device-farm testing for performance regression on target hardware.

## Graphics API

**Vulkan**

- Low-level, explicit GPU control ideal for mobile performance.
- Rust bindings via crates such as `ash`, `vulkano`, or custom FFI.
- On iOS, Vulkan is provided through MoltenVK (Vulkan over Metal).
- SPIR-V shaders compiled through Rust-friendly pipelines (e.g., `shaderc-rs`).

## Engine Architecture

- **Entity-Component-System**: `bevy_ecs`, `hecs`, or a custom ECS tailored to the game's needs.
- **Math**: `glam` for SIMD-accelerated vectors, matrices, and quaternions.
- **Threading**: explicit job system (e.g., `rayon` or a custom task scheduler) with a clear main-thread / render-thread split; data parallelism confined to per-frame phases.

## Rendering Techniques

- Mobile-optimized rendering: LOD, occlusion culling, frustum culling, batching.
- Custom Vulkan pipelines written in Rust.
- SPIR-V shaders tailored for mobile GPUs.
- Efficient memory management using Rust's ownership model.

## Asset Optimization

- Use mobile texture compression formats:
  - **ASTC** as the primary format for both Android and iOS (supported on all iOS devices since the A8 chip and on Vulkan-capable Android devices).
  - **ETC2** as a fallback for older Android devices without ASTC support.
  - **PVRTC** only as a legacy iOS fallback.
- Mesh optimization and compact data structures.
- Rust-based asset pipelines for preprocessing and validation.

## Performance Tools

- ARM Mobile Studio
- Qualcomm Adreno Profiler
- Mali Graphics Debugger
- Rust profiling tools:
  - `perf`
  - `cargo-flamegraph`
  - `criterion` for microbenchmarks

## Additional Libraries

### Physics

- Lightweight Rust physics engines (e.g., `rapier`) or custom Rust implementations.

### Audio

- **Android**: low-latency audio via Oboe/AAudio through Rust bindings (e.g., `oboe-rs`).
- **iOS**: Core Audio / AVAudioEngine via FFI.
- `rodio` acceptable for desktop development builds and prototyping; it is not the mobile production path.

### Input

- Platform-specific input through Rust FFI:
  - Android NDK
  - iOS UIKit/Swift bridging

## Persistence & Data

- `serde`-based serialization for save games, settings, and player progress.
- Compact binary formats (e.g., `bincode` or `postcard`) for save data; human-readable formats (RON/JSON) for configuration and debugging.
- Save-data versioning to support migration across game updates.
- Platform-appropriate storage locations accessed via Android NDK / iOS file system APIs.

## Networking & Telemetry

- Optional multiplayer: lightweight, transport-agnostic networking layer (e.g., QUIC via `quinn` or raw UDP with a custom reliability layer) — only if multiplayer is planned.
- Crash reporting integrated at the platform layer (Android Crashlytics / iOS crash handlers) with Rust panic hooks forwarding native crashes.
- Optional analytics for gameplay and performance telemetry.

## Native UI Implementation Considerations

### Input Handling

- Touch and gesture input via native APIs exposed to Rust.
- Convert raw input into game-space coordinates.
- Integrate with Rust's event system.

### UI Rendering

- UI drawn as textured quads or meshes using Vulkan.
- Custom Rust UI framework for layout, batching, and rendering.

### Text Rendering

- Bitmap fonts, SDF fonts, or vector rendering.
- Rust-based font libraries (e.g., `fontdue`, `rusttype`).

### Event System

- Rust event dispatcher for button presses, gestures, and UI interactions.
- Coordinate-based hit detection.

## Controls and Input Handling

### Input Abstraction Layer

- Rust trait-based abstraction for touch, keyboard, and mouse.
- High-level game actions mapped from raw input.

### Mobile Touch Input

- Multi-touch gesture handling.
- Virtual joysticks and buttons rendered via Vulkan.

### Keyboard & Mouse

- External device support via platform callbacks.

### Input State Management

- Per-frame tracking: pressed, held, released.
- Optional buffering for responsiveness.

### Cross-Platform Considerations

- Unified Rust interface hiding platform differences.
- Dynamic device switching.

### Performance

- Centralized per-frame polling.
- Avoid allocations; use Rust's stack-based patterns.

## Summary

Switching to Rust provides:

- Memory safety
- High performance
- Modern tooling
- Cleaner abstractions

Vulkan remains the rendering backbone (natively on Android, via MoltenVK on iOS), while Rust enables safer, more maintainable systems for architecture, UI, input, rendering, persistence, and game[...]

This tech stack delivers a fully custom, highly optimized mobile 3D engine built with Rust and Vulkan.
