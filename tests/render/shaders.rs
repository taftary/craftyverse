use planet_crafter_engine::testing::{
    CHECKER_CELLS, GEOM_FRAG, GEOM_VERT, LATITUDE_BANDS, LIGHT_DIR, MASK_EDGE_WIDTH, STRIPE_BANDS,
    TEX_FRAG, TEX_VERT, TEXT_FRAG, TEXT_VERT, compile_spirv,
};

#[test]
fn all_shaders_compile_to_spirv() {
    for (source, stage) in [
        (GEOM_VERT, naga::ShaderStage::Vertex),
        (GEOM_FRAG, naga::ShaderStage::Fragment),
        (TEXT_VERT, naga::ShaderStage::Vertex),
        (TEXT_FRAG, naga::ShaderStage::Fragment),
        (TEX_VERT, naga::ShaderStage::Vertex),
        (TEX_FRAG, naga::ShaderStage::Fragment),
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

/// Drift guard: the GLSL effect constants in `TEX_FRAG` must match the Rust
/// reference in `render::procedural`.
#[test]
fn tex_frag_constants_match_procedural_reference() {
    assert!(
        TEX_FRAG.contains(&format!(
            "const float CHECKER_CELLS = {:.1};",
            CHECKER_CELLS as f32
        )),
        "CHECKER_CELLS drifted"
    );
    assert!(
        TEX_FRAG.contains(&format!(
            "const float STRIPE_BANDS = {:.1};",
            STRIPE_BANDS as f32
        )),
        "STRIPE_BANDS drifted"
    );
    assert!(
        TEX_FRAG.contains(&format!(
            "const float MASK_EDGE_WIDTH = {MASK_EDGE_WIDTH:.2};"
        )),
        "MASK_EDGE_WIDTH drifted"
    );
    assert!(
        TEX_FRAG.contains(&format!(
            "const float LATITUDE_BANDS = {:.1};",
            LATITUDE_BANDS as f32
        )),
        "LATITUDE_BANDS drifted"
    );
    // LIGHT_DIR is written out in the GLSL (normalize() is not a constant
    // expression there): assert the literal matches the Rust value.
    assert!(
        TEX_FRAG.contains(&format!(
            "const vec3 LIGHT_DIR = vec3({:.7}, {:.7}, {:.7});",
            LIGHT_DIR.x, LIGHT_DIR.y, LIGHT_DIR.z
        )),
        "LIGHT_DIR drifted"
    );
}
