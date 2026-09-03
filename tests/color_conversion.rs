//! The checked-in BT.601 constants follow their real-valued definitions.

extern crate alloc;

#[path = "../src/color.rs"]
mod color;

const REAL_COEFFICIENTS: [f64; 9] = [
    0.2567882352941177,
    0.5041294117647058,
    0.09790588235294118,
    -0.1482235294117647,
    -0.290993,
    0.4392156862745098,
    0.4392156862745098,
    -0.3677882352941176,
    -0.07142745098039215,
];

#[test]
fn the_fixed_point_color_coefficients_regenerate_from_bt_601() {
    let generated = REAL_COEFFICIENTS.map(|value| (value * 65536.0).round() as i32);
    assert_eq!(generated, color::BT601_COEFFICIENTS);

    let planes = color::convert(&[0, 0, 0], 1, 1, 3);
    assert_eq!((planes.y_stride, planes.chroma_stride), (16, 8));
}
