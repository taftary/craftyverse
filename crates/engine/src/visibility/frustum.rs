//! Camera frustum: six planes extracted from a view-projection matrix,
//! with a conservative sphere test.

use glam::{Mat4, Vec3, Vec4};

/// A bounding sphere, the conservative bounding volume of one chunk.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingSphere {
    /// Sphere center in world space.
    pub center: Vec3,
    /// Sphere radius in world units (includes any skirt margin).
    pub radius: f32,
}

impl BoundingSphere {
    /// A sphere around `center` with the given `radius`.
    pub fn new(center: Vec3, radius: f32) -> Self {
        BoundingSphere {
            center,
            radius: radius.max(0.0),
        }
    }

    /// The conservative sphere of a triangle chunk: centered on
    /// `center`, covering every corner plus `margin` (the skirt depth, so
    /// the downward crack-masking flanges stay inside the volume).
    pub fn from_triangle(center: Vec3, vertices: [Vec3; 3], margin: f32) -> Self {
        let corner_radius = vertices
            .iter()
            .map(|vertex| vertex.distance(center))
            .fold(0.0_f32, f32::max);
        BoundingSphere::new(center, corner_radius + margin.max(0.0))
    }
}

/// One normalized frustum plane: inside is `normal . p + offset >= 0`.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Plane {
    normal: Vec3,
    offset: f32,
}

impl Plane {
    /// Builds a normalized plane from `row` (xyz = normal, w = offset); a
    /// degenerate row yields a harmless always-inside plane.
    fn new(row: Vec4) -> Self {
        let normal = row.truncate();
        let length = normal.length();
        if length <= f32::EPSILON {
            return Plane {
                normal: Vec3::ZERO,
                offset: 1.0,
            };
        }
        Plane {
            normal: normal / length,
            offset: row.w / length,
        }
    }

    /// Signed distance of `point` to the plane (positive inside).
    fn distance(&self, point: Vec3) -> f32 {
        self.normal.dot(point) + self.offset
    }
}

/// The camera view volume as six planes (left, right, bottom, top, near,
/// far), extracted from a view-projection matrix with the Gribb-Hartmann
/// method. The matrix convention matches the engine's cameras: right-handed
/// with a `0..=1` NDC depth range (the `directx` projection used by the
/// runtime window's fly camera), so the near plane is `z >= 0` in clip
/// space.
#[derive(Debug, Clone, Copy)]
pub struct Frustum {
    planes: [Plane; 6],
}

impl Frustum {
    /// Extracts the frustum of the view-projection matrix `mvp`
    /// (clip = mvp * world, y-up, NDC depth `0..=1`).
    pub fn from_view_projection(mvp: Mat4) -> Self {
        let row = |i: usize| mvp.row(i);
        let (r0, r1, r2, r3) = (row(0), row(1), row(2), row(3));
        Frustum {
            planes: [
                Plane::new(r3 + r0), // left
                Plane::new(r3 - r0), // right
                Plane::new(r3 + r1), // bottom
                Plane::new(r3 - r1), // top
                Plane::new(r2),      // near (z >= 0)
                Plane::new(r3 - r2), // far
            ],
        }
    }

    /// Whether `sphere` intersects or lies inside the frustum. A sphere is
    /// culled only when its closest point is outside one plane, so the
    /// test is conservative (err toward visible).
    pub fn contains_sphere(&self, sphere: &BoundingSphere) -> bool {
        self.planes
            .iter()
            .all(|plane| plane.distance(sphere.center) >= -sphere.radius)
    }
}
