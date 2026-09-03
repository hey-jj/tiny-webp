//! The checked-in BT.601 constants follow their real-valued definitions.

extern crate alloc;

#[path = "../src/color.rs"]
mod color;

#[test]
fn the_fixed_point_color_coefficients_regenerate_from_bt_601() {
    const KR: f64 = 0.299;
    const KG: f64 = 0.587;
    const KB: f64 = 0.114;
    const Y_SCALE: f64 = 219.0 / 255.0;
    const CHROMA_SCALE: f64 = 224.0 / 255.0;

    let real_coefficients = [
        KR * Y_SCALE,
        KG * Y_SCALE,
        KB * Y_SCALE,
        -KR * CHROMA_SCALE / (2.0 * (1.0 - KB)),
        -KG * CHROMA_SCALE / (2.0 * (1.0 - KB)),
        CHROMA_SCALE / 2.0,
        CHROMA_SCALE / 2.0,
        -KG * CHROMA_SCALE / (2.0 * (1.0 - KR)),
        -KB * CHROMA_SCALE / (2.0 * (1.0 - KR)),
    ];
    let generated = real_coefficients.map(|value| (value * 65536.0).round() as i32);
    assert_eq!(generated, color::BT601_COEFFICIENTS);

    let planes = color::convert(&[0, 0, 0], 1, 1, 3);
    assert_eq!((planes.y_stride, planes.chroma_stride), (16, 8));
}
