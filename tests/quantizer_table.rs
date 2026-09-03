//! The checked-in quality table follows its ruled formula.

#[path = "../src/quantize.rs"]
mod quantize;

#[test]
fn the_quality_lookup_regenerates_from_the_cube_root_formula() {
    let generated = core::array::from_fn(|quality| {
        let quality = quality as f64 / 100.0;
        let linear = if quality < 0.75 {
            quality * 2.0 / 3.0
        } else {
            2.0 * quality - 1.0
        };
        (127.0 * (1.0 - linear.cbrt())).floor() as u8
    });
    assert_eq!(generated, quantize::Q_TO_INDEX);
}
