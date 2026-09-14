//! The embedded GLSL shaders (geometry, text and textured pairs) and their
//! runtime compilation to SPIR-V through naga (pure Rust, no native shader
//! toolchain needed).

use std::sync::Arc;

use vulkano::device::Device;
use vulkano::shader::{EntryPoint, ShaderModule, ShaderModuleCreateInfo};

/// Geometry vertex shader: `mvp` push-constant view-projection transform,
/// passes the vertex color through.
pub const GEOM_VERT: &str = r#"
#version 450
layout(location = 0) in vec3 pos;
layout(location = 1) in vec3 color;
layout(location = 0) out vec3 out_color;
layout(push_constant) uniform PushMatrix { mat4 mvp; } pc;
void main() {
    gl_Position = pc.mvp * vec4(pos, 1.0);
    out_color = color;
}
"#;

/// Geometry fragment shader: flat interpolated color.
pub const GEOM_FRAG: &str = r#"
#version 450
layout(location = 0) in vec3 color;
layout(location = 0) out vec4 out_color;
void main() {
    out_color = vec4(color, 1.0);
}
"#;

/// Text vertex shader: same transform as the geometry shader, plus atlas UV.
pub const TEXT_VERT: &str = r#"
#version 450
layout(location = 0) in vec2 pos;
layout(location = 1) in vec2 uv;
layout(location = 2) in vec3 color;
layout(location = 0) out vec2 out_uv;
layout(location = 1) out vec3 out_color;
layout(push_constant) uniform PushTransform { vec2 scale; vec2 offset; } pc;
void main() {
    gl_Position = vec4(pos * pc.scale + pc.offset, 0.0, 1.0);
    out_uv = uv;
    out_color = color;
}
"#;

/// Text fragment shader: glyph coverage from the atlas in the alpha channel.
pub const TEXT_FRAG: &str = r#"
#version 450
layout(location = 0) in vec2 uv;
layout(location = 1) in vec3 color;
layout(location = 0) out vec4 out_color;
// Note: naga's GLSL frontend supports neither `layout(set = ...)` (resources
// default to set 0) nor combined `sampler2D` uniforms, so the glyph atlas is
// bound as a separate texture and sampler.
layout(binding = 0) uniform texture2D atlas_texture;
layout(binding = 1) uniform sampler atlas_sampler;
void main() {
    out_color = vec4(color, texture(sampler2D(atlas_texture, atlas_sampler), uv).r);
}
"#;

/// Textured vertex shader: `mvp` push-constant view-projection transform,
/// with the ground-flattening morph (feature 5, Decision 3 of
/// `plan/RELATED.md`) and the floating-origin compensation built in. The
/// shared `PushTex` block carries the anchor (the floating origin), the
/// outward radial at the anchor and the authoritative flatten factor next
/// to the camera position and the fragment mode. The vertex position is
/// first shifted into the anchor-relative frame (`pos - pc.anchor`, exact
/// for nearby f32 values - pooled vertex data stays spherical and
/// unmodified), projected onto the tangent plane at the anchor (the plane
/// through the anchor with normal `anchor_up`), and blended by `flatten`;
/// the `mvp` is built from the anchor-relative camera, so the whole
/// transform stays in small local coordinates. At `flatten == 0` with a
/// zero anchor (the debug viewer) the morph is the identity. Skirt vertices
/// morph with the same formula, so chunks and their crack-masking skirts
/// stay consistent at every blend value. The CPU mirror is
/// `render::flatten::morph_vertex` (test-internals). Passes the texture
/// coordinate, the barycentric coordinate, the parity sign, the morphed
/// (anchor-relative) position, the radial direction and the ring-field
/// value through.
pub const TEX_VERT: &str = r#"
#version 450
layout(location = 0) in vec3 pos;
layout(location = 1) in vec2 uv;
layout(location = 2) in vec3 bary;
layout(location = 3) in float parity;
layout(location = 4) in vec3 radial;
layout(location = 5) in float ring;
layout(location = 0) out vec2 out_uv;
layout(location = 1) out vec3 out_bary;
layout(location = 2) out float out_parity;
layout(location = 3) out vec3 out_world_pos;
layout(location = 4) out vec3 out_radial;
layout(location = 5) out float out_ring;
layout(push_constant) uniform PushTex {
    mat4 mvp;
    vec3 camera_pos;
    uint mode;
    vec3 anchor;
    vec3 anchor_up;
    float flatten;
} pc;
void main() {
    // Floating origin + ground-flattening morph: shift into the
    // anchor-relative frame, project onto the tangent plane at the anchor
    // (normal anchor_up), blend by the authoritative flatten factor.
    vec3 local = pos - pc.anchor;
    vec3 flattened = local - pc.anchor_up * dot(local, pc.anchor_up);
    vec3 morphed = mix(local, flattened, pc.flatten);
    gl_Position = pc.mvp * vec4(morphed, 1.0);
    out_uv = uv;
    out_bary = bary;
    out_parity = parity;
    out_world_pos = morphed;
    out_radial = radial;
    out_ring = ring;
}
"#;

/// Textured fragment shader: mode 0 (the UV-map view) samples the
/// checkerboard texture; modes 1..=11 evaluate a procedural per-triangle
/// effect from the interpolated barycentric coordinates, the parity sign,
/// the radial direction and the ring field — never the UVs, so the
/// procedural output is seam-independent. The effect functions mirror
/// `render::procedural` formula-for-formula. Mode 8 (diffuse, the runtime
/// window's terrain mode) additionally applies the feature-8 ground cue:
/// a flatten-gated micro checker (visual only, no CPU mirror) plus a
/// flatten- and distance-gated haze toward the horizon color, so dense
/// near ground reads detailed and the far field melts into the haze.
pub const TEX_FRAG: &str = r#"
#version 450
layout(location = 0) in vec2 uv;
layout(location = 1) in vec3 bary;
layout(location = 2) in float parity;
layout(location = 3) in vec3 world_pos;
layout(location = 4) in vec3 radial;
layout(location = 5) in float ring;
layout(location = 0) out vec4 out_color;
// Note: naga's GLSL frontend supports neither `layout(set = ...)` (resources
// default to set 0) nor combined `sampler2D` uniforms, so the checkerboard is
// bound as a separate texture and sampler.
layout(binding = 0) uniform texture2D checkerboard_texture;
layout(binding = 1) uniform sampler checkerboard_sampler;
layout(push_constant) uniform PushTex {
    mat4 mvp;
    vec3 camera_pos;
    uint mode;
    vec3 anchor;
    vec3 anchor_up;
    float flatten;
} pc;

// Procedural effect constants: keep in sync with render::procedural (the
// render tests assert it). LIGHT_DIR is normalize(vec3(1, 1, 1)) written
// out (normalize() is not a GLSL constant expression).
const float CHECKER_CELLS = 8.0;
const float STRIPE_BANDS = 8.0;
const float MASK_EDGE_WIDTH = 0.15;
const vec3 LIGHT_DIR = vec3(0.5773503, 0.5773503, 0.5773503);
const float LATITUDE_BANDS = 12.0;

float apply_parity(float value, float parity_sign) {
    return parity_sign < 0.0 ? 1.0 - value : value;
}

float checker(vec3 bary_coords, float parity_sign) {
    vec3 q = floor(bary_coords * CHECKER_CELLS);
    return apply_parity(mod(q.x + q.y + q.z, 2.0), parity_sign);
}

float stripes(float u, float parity_sign) {
    return apply_parity(mod(floor(u * STRIPE_BANDS), 2.0), parity_sign);
}

float edge_mask(vec3 bary_coords, float parity_sign) {
    vec3 b = parity_sign < 0.0 ? vec3(bary_coords.x, bary_coords.z, bary_coords.y) : bary_coords;
    return b.z <= MASK_EDGE_WIDTH ? 1.0 : 0.0;
}

void main() {
    if (pc.mode == 0u) {
        out_color = vec4(texture(sampler2D(checkerboard_texture, checkerboard_sampler), uv).rgb, 1.0);
        return;
    }
    // `radial` carries the normalized direction_to_origin (inward on a
    // sphere); the radial effects use the outward surface normal.
    vec3 normal = -radial;
    vec3 color;
    if (pc.mode == 1u) {
        color = bary;
    } else if (pc.mode == 2u) {
        color = vec3(checker(bary, parity));
    } else if (pc.mode == 3u) {
        color = vec3(stripes(bary.z, parity));
    } else if (pc.mode == 4u) {
        color = vec3(stripes(bary.x, parity));
    } else if (pc.mode == 5u) {
        color = vec3(stripes(bary.y, parity));
    } else if (pc.mode == 6u) {
        color = vec3(edge_mask(bary, parity));
    } else if (pc.mode == 7u) {
        color = normal * 0.5 + 0.5;
    } else if (pc.mode == 8u) {
        // Runtime terrain mode (feature 8 ground cue, visual only): the
        // Lambert term carries a flatten-gated micro checker (re-tiling
        // per leaf, so finer LOD reads finer) and melts into horizon haze
        // with flatten-gated distance. At flatten 0 this is plain diffuse.
        float lambert = max(dot(normal, LIGHT_DIR), 0.0);
        float micro = mix(1.0, 0.85 + 0.15 * checker(bary, parity), pc.flatten);
        float dist = length(pc.camera_pos - world_pos);
        float haze = pc.flatten * smoothstep(50.0, 600.0, dist);
        vec3 ground = vec3(lambert * micro);
        color = mix(ground, vec3(0.75, 0.85, 0.95), haze * 0.55);
    } else if (pc.mode == 9u) {
        float t = dot(normal, vec3(0.0, 1.0, 0.0)) * 0.5 + 0.5;
        color = vec3(mod(floor(t * LATITUDE_BANDS), 2.0));
    } else if (pc.mode == 10u) {
        vec3 view_dir = normalize(pc.camera_pos - world_pos);
        color = vec3(1.0 - abs(dot(normal, view_dir)));
    } else {
        color = vec3(mod(floor(ring), 2.0));
    }
    out_color = vec4(color, 1.0);
}
"#;

/// Atmosphere vertex shader (feature 6): transforms the curved shell into
/// the anchor-relative frame (`pos - anchor`, the floating-origin
/// compensation - the shell vertex data is built once and never rewritten)
/// and projects it. Unlike `TEX_VERT` there is no ground-flattening morph:
/// the shell stays curved at all times, reading as a sky dome above the
/// flattened ground at full flattening (Decision 3 of `plan/RELATED.md`).
/// Passes the outward radial and the anchor-relative position through.
pub const ATMO_VERT: &str = r#"
#version 450
layout(location = 0) in vec3 pos;
layout(location = 1) in vec3 dir;
layout(location = 0) out vec3 out_dir;
layout(location = 1) out vec3 out_world_pos;
layout(push_constant) uniform PushAtmosphere {
    mat4 mvp;
    vec3 camera_pos;
    vec3 anchor;
    float atmosphere_factor;
    float rim_factor;
} pc;
void main() {
    vec3 local = pos - pc.anchor;
    gl_Position = pc.mvp * vec4(local, 1.0);
    out_dir = dir;
    out_world_pos = local;
}
"#;

/// Atmosphere fragment shader (feature 6): the single curved-atmosphere
/// shader, covering both the outside view (space/orbit rim, curved
/// scattering layer) and the inside view (sky dome with horizon haze over
/// the flattened ground). The appearance is driven only by the two
/// push-constant factors - the shader does no independent distance math:
/// `pc.atmosphere_factor` (0 at/beyond the shell edge, 1 at the surface)
/// parameterizes every layer transition, so there are no hard cuts, and
/// flags the inside view (`> 0` iff the camera is under the shell);
/// `pc.rim_factor` fades the limb glow in across the orbit layer. The
/// weight functions mirror `render::atmosphere` formula-for-formula; the
/// render tests assert the constants match.
pub const ATMO_FRAG: &str = r#"
#version 450
layout(location = 0) in vec3 dir;
layout(location = 1) in vec3 world_pos;
layout(location = 0) out vec4 out_color;
layout(push_constant) uniform PushAtmosphere {
    mat4 mvp;
    vec3 camera_pos;
    vec3 anchor;
    float atmosphere_factor;
    float rim_factor;
} pc;

// Atmosphere appearance constants: keep in sync with render::atmosphere
// (the render tests assert it).
const float RIM_POWER = 2.00;
const float RIM_MAX_ALPHA = 0.90;
const float RIM_FADE_END = 0.35;
const float SCATTER_RISE_END = 0.30;
const float SCATTER_FADE_START = 0.60;
const float SCATTER_FADE_END = 0.95;
const float SCATTER_MAX_ALPHA = 0.60;
const float DOME_RISE_START = 0.55;
const float DOME_RISE_END = 0.95;
const float DOME_HAZE = 0.85;
const vec3 RIM_COLOR = vec3(0.60, 0.78, 1.00);
const vec3 SCATTER_COLOR = vec3(0.35, 0.60, 1.00);
const vec3 SKY_COLOR = vec3(0.30, 0.55, 0.95);
const vec3 HORIZON_COLOR = vec3(0.75, 0.85, 0.95);

void main() {
    float f = pc.atmosphere_factor;
    vec3 view_dir = normalize(pc.camera_pos - world_pos);
    float cos_view = dot(normalize(dir), view_dir);
    float rim = max(1.0 - abs(cos_view), 0.0);
    float a_rim = pc.rim_factor * (1.0 - smoothstep(0.0, RIM_FADE_END, f))
        * pow(rim, RIM_POWER) * RIM_MAX_ALPHA;
    float w_scatter = smoothstep(0.0, SCATTER_RISE_END, f)
        * (1.0 - smoothstep(SCATTER_FADE_START, SCATTER_FADE_END, f));
    float a_scatter = w_scatter * (0.3 + 0.7 * pow(rim, 1.5)) * SCATTER_MAX_ALPHA;
    float a_dome = 0.0;
    vec3 dome_color = SKY_COLOR;
    if (f > 0.0) {
        // Inside the shell (the camera crossed it during the descent): the
        // curved shell reads as a sky dome, hazy near the horizon.
        float w_dome = smoothstep(DOME_RISE_START, DOME_RISE_END, f);
        float haze = (1.0 - max(cos_view, 0.0)) * DOME_HAZE;
        dome_color = mix(SKY_COLOR, HORIZON_COLOR, haze * (0.4 + 0.6 * w_dome));
        a_dome = w_dome;
    }
    float total = a_rim + a_scatter + a_dome;
    vec3 color = total > 1e-6
        ? (RIM_COLOR * a_rim + SCATTER_COLOR * a_scatter + dome_color * a_dome) / total
        : RIM_COLOR;
    out_color = vec4(color, min(total, 1.0));
}
"#;

/// Compiles GLSL `source` for `stage` to SPIR-V words: naga parse, validate,
/// write. Headless — no Vulkan device involved, so it is unit-testable.
pub fn compile_spirv(source: &str, stage: naga::ShaderStage) -> Result<Vec<u32>, String> {
    let module = naga::front::glsl::Frontend::default()
        .parse(&naga::front::glsl::Options::from(stage), source)
        .map_err(|err| format!("GLSL parse failed: {err}"))?;
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .map_err(|err| format!("shader validation failed: {err}"))?;
    naga::back::spv::write_vec(&module, &info, &naga::back::spv::Options::default(), None)
        .map_err(|err| format!("SPIR-V generation failed: {err}"))
}

/// Compiles GLSL to a `ShaderModule` at runtime through naga (pure Rust, no
/// native shader toolchain needed).
pub(crate) fn load_shader(
    device: &Arc<Device>,
    source: &str,
    stage: naga::ShaderStage,
) -> EntryPoint {
    let words = compile_spirv(source, stage).expect("shader compilation failed");
    // SAFETY: the SPIR-V comes from naga's validated output, so it satisfies
    // the validity invariants `ShaderModule::new` requires.
    unsafe { ShaderModule::new(device.clone(), ShaderModuleCreateInfo::new(&words)) }
        .expect("failed to create shader module")
        .entry_point("main")
        .expect("shader has no `main` entry point")
}
