//! Reusable vertex buffers for the scene geometry batches.
//!
//! `Renderer::set_scene` rebuilds the scene on every scenario, view-mode,
//! checkbox, split or resize change. Instead of dropping and recreating the
//! device buffers each time, a [`VertexBuffer`] keeps its allocation and
//! rewrites the live range through the host-visible mapping when the new
//! data fits, growing the allocation by doubling otherwise.

use std::sync::Arc;

use vulkano::buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator};

/// Allocation capacity for `needed` elements: `current` while it suffices,
/// else doubling growth capped below by the request.
///
/// `pub` inside the private `buffers` module; it only escapes under the
/// `test-internals` feature for the spec tests.
pub fn required_capacity(current: usize, needed: usize) -> usize {
    if needed <= current {
        current
    } else {
        needed.max(current.saturating_mul(2))
    }
}

/// A vertex buffer with spare capacity: [`update`](Self::update) rewrites
/// the live range in place when it fits the allocation, or reallocates with
/// [`required_capacity`] growth. The allocation may exceed the live vertex
/// count, so draws must use the count from [`batch`](Self::batch), never the
/// buffer length.
pub(crate) struct VertexBuffer<T: BufferContents> {
    buffer: Option<Subbuffer<[T]>>,
    /// Live vertex count of the current batch.
    len: u32,
    /// Allocated element count of `buffer`.
    capacity: usize,
}

impl<T: BufferContents + Copy> VertexBuffer<T> {
    /// An empty buffer: no allocation until the first non-empty `update`.
    pub(crate) fn new() -> Self {
        VertexBuffer {
            buffer: None,
            len: 0,
            capacity: 0,
        }
    }

    /// An empty buffer with a pre-allocated capacity of `capacity` elements:
    /// the allocation is stable from creation (pool slots never reallocate),
    /// and the live batch stays empty until the first non-empty `update`.
    pub(crate) fn with_capacity(allocator: &Arc<StandardMemoryAllocator>, capacity: usize) -> Self {
        if capacity == 0 {
            return VertexBuffer::new();
        }
        let buffer = Buffer::new_slice::<T>(
            allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            capacity as u64,
        )
        .expect("failed to create vertex buffer");
        VertexBuffer {
            buffer: Some(buffer),
            len: 0,
            capacity,
        }
    }

    /// Replaces the live range with `data`. Rewrites through the mapping
    /// when the allocation suffices and reallocates otherwise. An empty
    /// `data` empties the batch but keeps the allocation for reuse.
    ///
    /// The caller must guarantee the GPU no longer reads the buffer;
    /// `Renderer::set_scene` waits for the device before calling this.
    pub(crate) fn update(&mut self, allocator: &Arc<StandardMemoryAllocator>, data: &[T]) {
        self.len = data.len() as u32;
        if data.is_empty() {
            return;
        }
        if data.len() > self.capacity {
            self.capacity = required_capacity(self.capacity, data.len());
            self.buffer = Some(
                Buffer::new_slice::<T>(
                    allocator.clone(),
                    BufferCreateInfo {
                        usage: BufferUsage::VERTEX_BUFFER,
                        ..Default::default()
                    },
                    AllocationCreateInfo {
                        memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                            | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                        ..Default::default()
                    },
                    self.capacity as u64,
                )
                .expect("failed to create vertex buffer"),
            );
        }
        self.buffer
            .as_ref()
            .expect("vertex buffer missing after allocation")
            .write()
            .expect("failed to map vertex buffer")[..data.len()]
            .copy_from_slice(data);
    }

    /// The live batch `(buffer, vertex_count)`, or `None` when empty (the
    /// matching draw batch is then skipped).
    pub(crate) fn batch(&self) -> Option<(Subbuffer<[T]>, u32)> {
        let buffer = self.buffer.as_ref()?;
        (self.len > 0).then(|| (buffer.clone(), self.len))
    }
}
