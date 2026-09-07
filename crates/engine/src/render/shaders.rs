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
/// geometry shader, passes the texture coordinate through.
pub const TEX_VERT: &str = r#"
#version 450
layout(location = 0) in vec3 pos;
layout(location = 1) in vec2 uv;
layout(location = 0) out vec2 out_uv;
layout(push_constant) uniform PushMatrix { mat4 mvp; } pc;
void main() {
    gl_Position = pc.mvp * vec4(pos, 1.0);
    out_uv = uv;
}
"#;

/// Textured fragment shader: opaque color sampled from the checkerboard.
pub const TEX_FRAG: &str = r#"
#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 out_color;
// Note: naga's GLSL frontend supports neither `layout(set = ...)` (resources
// default to set 0) nor combined `sampler2D` uniforms, so the checkerboard is
// bound as a separate texture and sampler.
layout(binding = 0) uniform texture2D checkerboard_texture;
layout(binding = 1) uniform sampler checkerboard_sampler;
void main() {
    out_color = vec4(texture(sampler2D(checkerboard_texture, checkerboard_sampler), uv).rgb, 1.0);
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
