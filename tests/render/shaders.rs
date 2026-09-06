use planet_crafter_engine::testing::{GEOM_FRAG, GEOM_VERT, TEXT_FRAG, TEXT_VERT, compile_spirv};

#[test]
fn all_shaders_compile_to_spirv() {
    for (source, stage) in [
        (GEOM_VERT, naga::ShaderStage::Vertex),
        (GEOM_FRAG, naga::ShaderStage::Fragment),
        (TEXT_VERT, naga::ShaderStage::Vertex),
        (TEXT_FRAG, naga::ShaderStage::Fragment),
    ] {
        let words = compile_spirv(source, stage).expect("shader should compile");
        // SPIR-V magic number.
        assert_eq!(words[0], 0x0723_0203);
    }
}

#[test]
fn compile_spirv_rejects_invalid_glsl() {
    assert!(compile_spirv("void main() {", naga::ShaderStage::Vertex).is_err());
}
