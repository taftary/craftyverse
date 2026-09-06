//! Orbit camera: yaw/pitch/zoom state, the bounding-sphere view fit and the
//! world-label projection. Pure glam math — no GPU code — so every piece is
//! unit-testable.

use glam::camera::rh::proj::vulkan;
use glam::camera::rh::view::look_at_mat4;
use glam::{Mat4, Vec2, Vec3};

use super::{LabelOffset, TextRun, WorldLabel};

/// Vertical field of view of the perspective projection (45°).
const FIELD_OF_VIEW: f32 = std::f32::consts::FRAC_PI_4;

/// Extra margin factor of the bounding-sphere fit.
const FIT_MARGIN: f32 = 1.05;

/// Lower clamp of the fit radius, keeping the camera distance finite for
/// point-like scenes.
pub(crate) const MIN_FIT_RADIUS: f32 = 1e-3;

/// Pitch clamp: the camera never quite reaches the poles, where the up
/// vector would be parallel to the view direction.
pub const MAX_PITCH: f32 = std::f32::consts::FRAC_PI_2 - 0.01;

/// Lower zoom clamp (magnification factor on the fitted camera distance).
pub const MIN_ZOOM: f32 = 0.05;
/// Upper zoom clamp (magnification factor on the fitted camera distance).
pub const MAX_ZOOM: f32 = 20.0;

/// Orbit camera around the content bounding sphere.
///
/// Angles are in radians; `zoom` is a magnification factor on the fitted
/// camera distance. The default camera (yaw 0, pitch 0, zoom 1) sits on the
/// +Z axis of the content center — the head-on XY view. Dragging orbits the
/// view so the content follows the cursor (see
/// [`OrbitCamera::view_projection`] for the exact frame).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrbitCamera {
    yaw: f32,
    pitch: f32,
    zoom: f32,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.0,
            zoom: 1.0,
        }
    }
}

impl OrbitCamera {
    /// Current yaw (rotation around the world Y axis), in radians.
    pub fn yaw(&self) -> f32 {
        self.yaw
    }

    /// Current pitch (elevation above/below the equator), in radians.
    pub fn pitch(&self) -> f32 {
        self.pitch
    }

    /// Current zoom magnification factor.
    pub fn zoom(&self) -> f32 {
        self.zoom
    }

    /// Adds `delta_yaw` / `delta_pitch` (radians) to the orbit angles; the
    /// pitch is clamped to just short of the poles.
    pub fn orbit(&mut self, delta_yaw: f32, delta_pitch: f32) {
        self.yaw += delta_yaw;
        self.pitch = (self.pitch + delta_pitch).clamp(-MAX_PITCH, MAX_PITCH);
    }

    /// Multiplies the zoom by `factor`, clamped to `0.05..=20.0`.
    pub fn zoom_by(&mut self, factor: f32) {
        self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
    }

    /// Returns to the default head-on view.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// View-projection matrix fitting the content sphere (`center`, `radius`)
    /// into `viewport` pixels.
    ///
    /// The camera sits on the yaw/pitch sphere around `center` at a distance
    /// that frames the whole content sphere in the smaller of the
    /// horizontal/vertical fields of view (plus a 5% margin), scaled by
    /// `1 / zoom`. The viewport is clamped to at least 1×1 pixels and
    /// `radius` to `MIN_FIT_RADIUS`, keeping the distance finite. The near
    /// plane sits at `distance - 2 * radius` (clamped to a small positive
    /// value), the far plane at `distance + 2 * radius`. At identity angles
    /// the eye is at `center + Z * distance` looking at `center` with up +Y,
    /// so world +X appears right and world +Y up. The projection uses
    /// Vulkan's clip convention (depth `z ∈ [0,1]`, y-down NDC) so world up
    /// lands at the top of the viewport.
    pub fn view_projection(&self, center: Vec3, radius: f32, viewport: Vec2) -> Mat4 {
        let viewport = viewport.max(Vec2::ONE);
        let radius = radius.max(MIN_FIT_RADIUS);
        let aspect = viewport.x / viewport.y;
        let half_fov_y = FIELD_OF_VIEW * 0.5;
        let half_fov_x = (half_fov_y.tan() * aspect).atan();
        let distance = radius / half_fov_y.min(half_fov_x).sin() * FIT_MARGIN / self.zoom;
        let near = (distance - 2.0 * radius).max(radius * 1e-3);
        let far = distance + 2.0 * radius;
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let eye =
            center + Vec3::new(sin_yaw * cos_pitch, sin_pitch, cos_yaw * cos_pitch) * distance;
        let view = look_at_mat4(eye, center, Vec3::Y);
        vulkan::perspective(FIELD_OF_VIEW, aspect, near, far) * view
    }
}

/// Projects `labels` to pixel space through `mvp` (from
/// [`OrbitCamera::view_projection`]) and `viewport` pixels, resolving each
/// label's pixel offset. Labels behind the camera are dropped; a
/// [`LabelOffset::Outward`] label whose reference point is behind the
/// camera is dropped too, and one whose projected anchor coincides with
/// the projected reference point is pushed straight up (`(0, -1)`). The
/// returned runs borrow their text from the labels.
pub fn project_labels<'a>(
    labels: &'a [WorldLabel],
    mvp: &Mat4,
    viewport: Vec2,
) -> Vec<TextRun<'a>> {
    let to_pixel = |point: Vec3| -> Option<Vec2> {
        let clip = *mvp * point.extend(1.0);
        if clip.w <= 0.0 {
            return None;
        }
        let ndc = clip.truncate() / clip.w;
        Some(Vec2::new(
            (ndc.x + 1.0) * 0.5 * viewport.x,
            (ndc.y + 1.0) * 0.5 * viewport.y,
        ))
    };
    labels
        .iter()
        .filter_map(|label| {
            let anchor = to_pixel(label.world_pos)?;
            let offset = match label.offset {
                LabelOffset::Fixed(offset) => offset,
                LabelOffset::Outward { from, distance_px } => {
                    let from_px = to_pixel(from)?;
                    (anchor - from_px)
                        .try_normalize()
                        .unwrap_or(Vec2::new(0.0, -1.0))
                        * distance_px
                }
            };
            Some(TextRun {
                text: &label.text,
                anchor: anchor + offset,
                size: label.size_px,
                color: label.color,
                centered: label.centered,
            })
        })
        .collect()
}
