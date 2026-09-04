//! Validated resource construction.
//!
//! Demonstrates error handling and type-driven resource construction: a font
//! atlas is built from raw bytes and fails explicitly on invalid input.
//!
//! See `docs/rust/book/practices/error-handling.md` and
//! `docs/rust/book/principles/type-driven-design.md`.

use planet_crafter_engine::text::TextAtlas;

/// Bundled font bytes used by the text-layout examples and tests.
const FONT: &[u8] = include_bytes!("../../../../assets/fonts/JetBrainsMono-Regular.ttf");

/// Runs the validated-resource demonstration.
///
/// Construction succeeds for a valid TTF and returns a descriptive error for
/// invalid bytes.
pub fn run() {
    let atlas = TextAtlas::new(FONT).expect("bundled font is valid");
    assert_eq!(atlas.width, 512);
    assert!(!atlas.pixels.is_empty());

    let bad = TextAtlas::new(b"not a font");
    assert!(bad.is_err());
}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn validated_resource_demo_runs() {
        run();
    }
}
