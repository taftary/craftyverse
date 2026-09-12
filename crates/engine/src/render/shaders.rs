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

/// Textured vertex shader: same `mvp` push-constant transform as the
/// geometry shader (the shared `PushTex` block also carries the camera
/// position and the fragment mode, unused here), passes the texture
/// coordinate, the barycentric coordinate, the parity sign, the world
/// position, the radial direction and the ring-field value through.
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
layout(push_constant) uniform PushTex { mat4 mvp; vec3 camera_pos; uint mode; } pc;
void main() {
    gl_Position = pc.mvp * vec4(pos, 1.0);
    out_uv = uv;
    out_bary = bary;
    out_parity = parity;
    out_world_pos = pos;
    out_radial = radial;
    out_ring = ring;
}
"#;

/// Textured fragment shader: mode 0 (the UV-map view) samples the
/// checkerboard texture; modes 1..=11 evaluate a procedural per-triangle
/// effect from the interpolated barycentric coordinates, the parity sign,
/// the radial direction and the ring field — never the UVs, so the
/// procedural output is seam-independent. The effect functions mirror
/// `render::procedural` formula-for-formula.
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
layout(push_constant) uniform PushTex { mat4 mvp; vec3 camera_pos; uint mode; } pc;

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
        color = vec3(max(dot(normal, LIGHT_DIR), 0.0));
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
