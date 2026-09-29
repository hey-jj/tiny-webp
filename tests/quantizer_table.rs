//! The checked-in quality table regenerates from its formula.

#![forbid(unsafe_code)]

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

#[test]
fn every_bit_cost_regenerates_from_the_binary_logarithm() {
    let generated = core::array::from_fn(|probability| {
        if probability == 0 {
            2048
        } else {
            (-256.0 * (probability as f64 / 256.0).log2()).round() as u16
        }
    });
    assert_eq!(quantize::BIT_COST.len(), 256);
    assert_eq!(quantize::BIT_COST[0], 2048);
    assert_eq!(quantize::BIT_COST[1], 2048);
    assert_eq!(quantize::BIT_COST[255], 1);
    assert_eq!(generated, quantize::BIT_COST);
}
