use planet_crafter_engine::testing::{
    ATMO_FRAG, ATMO_VERT, CHECKER_CELLS, DOME_HAZE, DOME_RISE_END, DOME_RISE_START, GEOM_FRAG,
    GEOM_VERT, HORIZON_COLOR, LATITUDE_BANDS, LIGHT_DIR, MASK_EDGE_WIDTH, RIM_COLOR, RIM_FADE_END,
    RIM_MAX_ALPHA, RIM_POWER, SCATTER_COLOR, SCATTER_FADE_END, SCATTER_FADE_START,
    SCATTER_MAX_ALPHA, SCATTER_RISE_END, SKY_COLOR, STRIPE_BANDS, TEX_FRAG, TEX_VERT, TEXT_FRAG,
    TEXT_VERT, compile_spirv,
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
        (ATMO_VERT, naga::ShaderStage::Vertex),
        (ATMO_FRAG, naga::ShaderStage::Fragment),
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

/// Drift guard: the GLSL appearance constants in `ATMO_FRAG` must match the
/// Rust reference in `render::atmosphere`.
#[test]
fn atmo_frag_constants_match_atmosphere_reference() {
    for (name, value) in [
        ("RIM_POWER", RIM_POWER),
        ("RIM_MAX_ALPHA", RIM_MAX_ALPHA),
        ("RIM_FADE_END", RIM_FADE_END),
        ("SCATTER_RISE_END", SCATTER_RISE_END),
        ("SCATTER_FADE_START", SCATTER_FADE_START),
        ("SCATTER_FADE_END", SCATTER_FADE_END),
        ("SCATTER_MAX_ALPHA", SCATTER_MAX_ALPHA),
        ("DOME_RISE_START", DOME_RISE_START),
        ("DOME_RISE_END", DOME_RISE_END),
        ("DOME_HAZE", DOME_HAZE),
    ] {
        assert!(
            ATMO_FRAG.contains(&format!("const float {name} = {value:.2};")),
            "{name} drifted"
        );
    }
    for (name, value) in [
        ("RIM_COLOR", RIM_COLOR),
        ("SCATTER_COLOR", SCATTER_COLOR),
        ("SKY_COLOR", SKY_COLOR),
        ("HORIZON_COLOR", HORIZON_COLOR),
    ] {
        assert!(
            ATMO_FRAG.contains(&format!(
                "const vec3 {name} = vec3({:.2}, {:.2}, {:.2});",
                value[0], value[1], value[2]
            )),
            "{name} drifted"
        );
    }
}
