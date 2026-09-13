//! Curved atmosphere shell (feature 6 of `plan/features/06-atmosphere.md`):
//! the shell geometry generator and the CPU reference implementation of the
//! atmosphere appearance the `shaders::ATMO_FRAG` fragment shader evaluates
//! per pixel.
//!
//! The shell is a plain lat-long triangle sphere centered on the planet
//! origin at the atmosphere shell radius
//! ([`PlanetConfig::atmosphere_radius`](crate::runtime::PlanetConfig::atmosphere_radius)).
//! It stays curved at all times, including at full ground flattening
//! (Decision 3 of `plan/RELATED.md`): the ground-flattening morph of
//! `shaders::TEX_VERT` is never applied to it — the shell vertex shader only
//! shifts vertices into the anchor-relative frame (`pos - anchor`), so the
//! pooled shell vertex data is built once and never rewritten per frame.
//!
//! The appearance is driven by two normalized factors the runtime window
//! derives from the authoritative [`RuntimeState`](crate::runtime::RuntimeState):
//!
//! - `atmosphere_factor` (0 at and beyond the shell edge, 1 at the surface)
//!   parameterizes every layer transition, so there are no hard cuts. Note
//!   `atmosphere_factor > 0` iff the camera is inside the shell, so the same
//!   shader covers the outside (rim, scattering from space and orbit) and
//!   the inside (sky dome over the flattened ground) views.
//! - `rim_factor` (0 in space, ramping to 1 across the orbit layer, 1 at and
//!   inside the shell) fades the limb glow in across the orbit layer; the
//!   manager's factor stays 0 there, so this ramp is derived on the CPU.
//!
//! The appearance functions below are the CPU reference of the GLSL mirror
//! in `shaders::ATMO_FRAG` (same formulas, same constants; the render tests
//! assert the constants match). They are test-only, gated behind the
//! `test-internals` feature like `render::procedural`: the runtime evaluates
//! the shader, never these functions. The shell geometry generator and the
//! `rim_factor` ramp are ungated — the runtime window uses both.

use glam::Vec3;

use crate::runtime::PlanetaryLayer;

/// Longitude segments of the atmosphere shell grid.
pub(crate) const SHELL_SLICES: u32 = 64;
/// Latitude segments of the atmosphere shell grid (pole to pole).
pub(crate) const SHELL_STACKS: u32 = 32;

/// One vertex of the atmosphere shell: the world-space position on the
/// shell sphere and the outward radial direction (the sphere normal,
/// normalized `pos - origin`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShellVertex {
    /// World-space position, exactly on the shell sphere.
    pub pos: Vec3,
    /// Outward radial direction (unit length).
    pub dir: Vec3,
}

/// Generates the atmosphere shell triangle vertices: a lat-long sphere of
/// `radius` around `origin` with `slices` longitude and `stacks` latitude
/// segments, two triangles per grid quad (`slices * stacks * 6` vertices;
/// empty when either segment count is 0). Every vertex lies exactly on the
/// shell sphere and carries the outward radial. The output is built once at
/// startup and never changes; per-frame rendering shifts it into the
/// anchor-relative frame in the vertex shader, and the ground-flattening
/// morph is never applied (the shell stays curved at all times).
pub fn shell_vertices(origin: Vec3, radius: f32, slices: u32, stacks: u32) -> Vec<ShellVertex> {
    if slices == 0 || stacks == 0 {
        return Vec::new();
    }
    let point = |slice: u32, stack: u32| {
        let theta = std::f32::consts::PI * stack as f32 / stacks as f32;
        let phi = 2.0 * std::f32::consts::PI * slice as f32 / slices as f32;
        let (sin_theta, cos_theta) = theta.sin_cos();
        let (sin_phi, cos_phi) = phi.sin_cos();
        let dir = Vec3::new(sin_theta * cos_phi, cos_theta, sin_theta * sin_phi);
        ShellVertex {
            pos: origin + dir * radius,
            dir,
        }
    };
    let mut vertices = Vec::with_capacity((slices * stacks * 6) as usize);
    for stack in 0..stacks {
        for slice in 0..slices {
            let next_slice = (slice + 1) % slices;
            let a = point(slice, stack);
            let b = point(next_slice, stack);
            let c = point(slice, stack + 1);
            let d = point(next_slice, stack + 1);
            vertices.extend([a, b, c]);
            vertices.extend([b, d, c]);
        }
    }
    vertices
}

/// The orbit-layer rim ramp: 0 at and beyond the orbit radius (space), 1 at
/// and inside the atmosphere shell radius, linear in between. The manager's
/// `atmosphere_factor` is clamped to 0 across the whole orbit layer, so the
/// thin rim of the orbit layer fades in through this CPU-side ramp instead —
/// the shader performs no independent distance math.
pub fn rim_factor(distance_to_center: f32, orbit_radius: f32, shell_radius: f32) -> f32 {
    ((orbit_radius - distance_to_center) / (orbit_radius - shell_radius)).clamp(0.0, 1.0)
}

/// The debug-overlay name of the atmosphere appearance of one planetary
/// layer: `none` (space), `rim` (orbit), `scattering` (atmosphere), `fog`
/// (sky), `sky dome` (terrain).
pub fn atmosphere_state_name(layer: PlanetaryLayer) -> &'static str {
    match layer {
        PlanetaryLayer::Space => "none",
        PlanetaryLayer::Orbit => "rim",
        PlanetaryLayer::Atmosphere => "scattering",
        PlanetaryLayer::Sky => "fog",
        PlanetaryLayer::Terrain => "sky dome",
    }
}

/// Fresnel exponent of the rim (limb glow) falloff.
#[cfg(feature = "test-internals")]
pub const RIM_POWER: f32 = 2.0;
/// Maximum alpha of the rim contribution.
#[cfg(feature = "test-internals")]
pub const RIM_MAX_ALPHA: f32 = 0.9;
/// Atmosphere factor at which the rim has fully faded out.
#[cfg(feature = "test-internals")]
pub const RIM_FADE_END: f32 = 0.35;
/// Atmosphere factor at which the scattering layer has fully risen.
#[cfg(feature = "test-internals")]
pub const SCATTER_RISE_END: f32 = 0.3;
/// Atmosphere factor at which the scattering layer starts fading out.
#[cfg(feature = "test-internals")]
pub const SCATTER_FADE_START: f32 = 0.6;
/// Atmosphere factor at which the scattering layer has fully faded out.
#[cfg(feature = "test-internals")]
pub const SCATTER_FADE_END: f32 = 0.95;
/// Maximum alpha of the scattering contribution.
#[cfg(feature = "test-internals")]
pub const SCATTER_MAX_ALPHA: f32 = 0.6;
/// Atmosphere factor at which the sky dome starts rising (inside view).
#[cfg(feature = "test-internals")]
pub const DOME_RISE_START: f32 = 0.55;
/// Atmosphere factor at which the sky dome is fully opaque (inside view).
#[cfg(feature = "test-internals")]
pub const DOME_RISE_END: f32 = 0.95;
/// Strength of the horizon haze of the sky dome (0 = none, 1 = full horizon
/// color at the horizon).
#[cfg(feature = "test-internals")]
pub const DOME_HAZE: f32 = 0.85;
/// Color of the rim (limb glow), a pale blue-white.
#[cfg(feature = "test-internals")]
pub const RIM_COLOR: [f32; 3] = [0.6, 0.78, 1.0];
/// Color of the curved scattering layer, a sky blue.
#[cfg(feature = "test-internals")]
pub const SCATTER_COLOR: [f32; 3] = [0.35, 0.6, 1.0];
/// Zenith color of the sky dome.
#[cfg(feature = "test-internals")]
pub const SKY_COLOR: [f32; 3] = [0.3, 0.55, 0.95];
/// Horizon color of the sky dome haze.
#[cfg(feature = "test-internals")]
pub const HORIZON_COLOR: [f32; 3] = [0.75, 0.85, 0.95];

/// GLSL `smoothstep`: 0 below `e0`, 1 above `e1`, cubic Hermite in between.
/// Continuous in `x`, so every appearance term built on it is continuous.
#[cfg(feature = "test-internals")]
fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(feature = "test-internals")]
fn lerp_color(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// The alpha (contribution weight) of the fresnel rim: strongest at the
/// limb (`rim` = 1), faded in across the orbit layer by `rim_factor` and
/// faded out by [`RIM_FADE_END`] as the descent continues.
#[cfg(feature = "test-internals")]
pub fn rim_weight(factor: f32, rim_factor: f32, rim: f32) -> f32 {
    rim_factor * (1.0 - smoothstep(0.0, RIM_FADE_END, factor)) * rim.powf(RIM_POWER) * RIM_MAX_ALPHA
}

/// The weight of the curved scattering layer: rises from the shell edge to
/// [`SCATTER_RISE_END`], holds through the atmosphere layer, fades out
/// across the sky layer into [`SCATTER_FADE_END`].
#[cfg(feature = "test-internals")]
pub fn scatter_weight(factor: f32) -> f32 {
    smoothstep(0.0, SCATTER_RISE_END, factor)
        * (1.0 - smoothstep(SCATTER_FADE_START, SCATTER_FADE_END, factor))
}

/// The weight (and alpha) of the sky dome, inside view only: 0 in the
/// atmosphere layer, fully opaque at and below the surface.
#[cfg(feature = "test-internals")]
pub fn dome_weight(factor: f32) -> f32 {
    smoothstep(DOME_RISE_START, DOME_RISE_END, factor)
}

/// One evaluated atmosphere fragment: the blended color and alpha.
#[cfg(feature = "test-internals")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AtmosphereSample {
    /// The blended atmosphere color.
    pub color: [f32; 3],
    /// The fragment alpha, `0..=1`.
    pub alpha: f32,
}

/// The atmosphere appearance of one fragment, mirroring `shaders::ATMO_FRAG`
/// formula-for-formula. `cos_view` is the cosine between the shell outward
/// radial and the direction from the fragment to the camera; `factor` is
/// the manager's `atmosphere_factor` (0 at/beyond the shell edge, 1 at the
/// surface); `rim_factor` is the orbit-layer rim ramp ([`rim_factor`]);
/// `inside` is whether the camera is under the shell (exactly
/// `factor > 0` — the shader derives the flag from the factor).
///
/// Every term is a smooth function of the factors, so the appearance is
/// continuous across the whole descent, including the shell crossing (at
/// `factor == 0` the inside dome term is 0, so the inside and outside views
/// agree) and every layer boundary. Space (`rim_factor == 0`,
/// `factor == 0`) is invisible.
#[cfg(feature = "test-internals")]
pub fn appearance(cos_view: f32, factor: f32, rim_factor: f32, inside: bool) -> AtmosphereSample {
    let rim = (1.0 - cos_view.abs()).max(0.0);
    let a_rim = rim_weight(factor, rim_factor, rim);
    let a_scatter = scatter_weight(factor) * (0.3 + 0.7 * rim.powf(1.5)) * SCATTER_MAX_ALPHA;
    let (a_dome, dome_color) = if inside {
        let w_dome = dome_weight(factor);
        let haze = (1.0 - cos_view.max(0.0)) * DOME_HAZE;
        let color = lerp_color(SKY_COLOR, HORIZON_COLOR, haze * (0.4 + 0.6 * w_dome));
        (w_dome, color)
    } else {
        (0.0, SKY_COLOR)
    };
    let total = a_rim + a_scatter + a_dome;
    let color = if total > 1e-6 {
        [
            (RIM_COLOR[0] * a_rim + SCATTER_COLOR[0] * a_scatter + dome_color[0] * a_dome) / total,
            (RIM_COLOR[1] * a_rim + SCATTER_COLOR[1] * a_scatter + dome_color[1] * a_dome) / total,
            (RIM_COLOR[2] * a_rim + SCATTER_COLOR[2] * a_scatter + dome_color[2] * a_dome) / total,
        ]
    } else {
        RIM_COLOR
    };
    AtmosphereSample {
        color,
        alpha: total.min(1.0),
    }
}
