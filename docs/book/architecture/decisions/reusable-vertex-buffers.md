# ADR: Reusable vertex buffers for scene rebuilds

**Status:** Accepted

**Deciders:** PlanetCrafter contributors

**Date:** 2026-09-11

## Context

The viewer rebuilds the scene mesh and re-uploads the geometry vertex
buffers on every scenario switch, view-mode switch, checkbox toggle, split,
and resize. Each rebuild dropped the seven geometry buffers and allocated
new ones, so routine interactions (toggling an attribute, cycling the view)
paid a full set of fresh device allocations for data that is usually similar
in size to the previous generation.

## Decision

Keep the geometry vertex buffers in a reusable `VertexBuffer<T>` wrapper
(`render/buffers.rs`): an update rewrites the live range through the
host-visible mapping while the new data fits the allocation, and reallocates
with doubling growth (`required_capacity`) otherwise. Empty batches keep
their allocation and are skipped at draw time. Because allocations now carry
spare capacity, draws use the stored live vertex count, never the buffer
length.

Synchronization has two parts before each `set_scene` update batch: a
device wait (`Device::wait_idle`, the third `unsafe` block in the render
module) so the rewrites cannot race in-flight work - the renderer is
single-threaded on the winit event loop, so nothing submits while it waits,
and `set_scene` runs only on discrete events, never per frame - and a
`cleanup_finished` on the previous frame's fence future right after the
wait, which releases vulkano's per-buffer in-flight bookkeeping (the fence
is known-signaled, so `signal_finished` propagates and the buffers unlock
for host writes; the wait alone does not).

The small text buffer is the exception: it rebuilds per camera change (the
hot path), where a device wait per camera event would stall orbiting. It
keeps reallocating, which is already safe through vulkano's deferred
destruction.

## Consequences

- Steady-state interactions (checkbox toggles, view-mode cycles, splits at
  a stable size) perform zero device allocations - one mapped copy per
  batch.
- Growth is amortized: each buffer reallocates at most O(log n) times as
  the scene grows, and shrinkage is free (capacity is kept).
- A device-wide wait is heavier than per-buffer fences, but it runs at
  event rate (worst case once per key press or resize step) where the GPU
  is almost always already idle; the simplicity was preferred over
  generation-tracked fencing.
- Callers of `record_draw` must pass the batch's vertex count explicitly;
  drawing `buffer.len()` would render uninitialized spare capacity.

## Alternatives considered

- **Keep reallocating per rebuild:** correct but wasteful; the churn is
  proportional to scene size for every minor toggle.
- **Per-buffer fences or frame-in-flight resource sets:** finer-grained
  sync without a device wait, but multiplies the bookkeeping for a debug
  viewer that is idle between events; rejected as disproportionate
  complexity.
- **Vertex welding / indexed meshes:** would cut memory roughly 3x, but
  changes the scene data model (per-corner UV seams need splits anyway) and
  is orthogonal to allocation churn; deferred.

## Related

- [Render specification](../../specs/render.md)
- [Vulkan crate and ownership model](vulkan-crate.md)
