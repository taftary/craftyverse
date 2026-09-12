use glam::Vec2;
use planet_crafter_engine::scene::{Attribute, PanelItem, PanelRow, TextureEffect};
use planet_crafter_engine::testing::panel_item_at;

#[test]
fn panel_item_at_hit_tests_pixel_rects() {
    let rows = [
        PanelRow {
            item: PanelItem::Attribute(Attribute::Outline),
            min: Vec2::new(0.0, 0.0),
            max: Vec2::new(100.0, 20.0),
        },
        PanelRow {
            item: PanelItem::Effect(TextureEffect::Checkerboard),
            min: Vec2::new(0.0, 24.0),
            max: Vec2::new(100.0, 44.0),
        },
    ];

    assert_eq!(
        panel_item_at(&rows, Vec2::new(50.0, 10.0)),
        Some(PanelItem::Attribute(Attribute::Outline))
    );
    assert_eq!(
        panel_item_at(&rows, Vec2::new(50.0, 30.0)),
        Some(PanelItem::Effect(TextureEffect::Checkerboard))
    );
    // Between the two rows, and right of both: no hit.
    assert_eq!(panel_item_at(&rows, Vec2::new(50.0, 22.0)), None);
    assert_eq!(panel_item_at(&rows, Vec2::new(150.0, 10.0)), None);
}
