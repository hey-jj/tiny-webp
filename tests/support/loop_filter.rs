//! Filters reconstructed key-frame planes using RFC 6386 section 15.

#[derive(Clone, Copy)]
pub(crate) struct Macroblock {
    pub(crate) skip: bool,
    pub(crate) b_pred: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Limits {
    interior: i32,
    macroblock: i32,
    subblock: i32,
    variance: i32,
}

impl Limits {
    fn new(level: u8, sharpness: u8) -> Self {
        let level = i32::from(level.min(63));
        let sharpness = sharpness.min(7);
        let mut interior = level;
        if sharpness != 0 {
            interior >>= if sharpness > 4 { 2 } else { 1 };
            interior = interior.min(9 - i32::from(sharpness));
        }
        interior = interior.max(1);
        Self {
            interior,
            macroblock: 2 * (level + 2) + interior,
            subblock: 2 * level + interior,
            variance: if level >= 40 {
                2
            } else if level >= 15 {
                1
            } else {
                0
            },
        }
    }
}

/// Filters padded Y, U, and V planes after reconstruction, per RFC 6386 section 15.1.
///
/// Plane rows span `columns * 16` luma pixels and `columns * 8` chroma pixels.
/// Each plane holds `rows` complete macroblock rows. Records use raster order.
///
/// # Panics
///
/// Panics at a positive level if plane or record lengths differ from the padded grid.
pub(crate) fn filter_frame(
    mut planes: [&mut [u8]; 3],
    columns: usize,
    rows: usize,
    level: u8,
    sharpness: u8,
    macroblocks: &[Macroblock],
) {
    // RFC 6386 section 15 requires a bypass at level zero.
    if level == 0 {
        return;
    }
    let limits = Limits::new(level, sharpness);
    assert_eq!(macroblocks.len(), columns * rows);
    for (plane, side) in planes.iter().zip([16, 8, 8]) {
        assert_eq!(plane.len(), columns * rows * side * side);
    }
    for row in 0..rows {
        for column in 0..columns {
            let block = macroblocks[row * columns + column];
            for (plane, side) in planes.iter_mut().zip([16, 8, 8]) {
                let stride = columns * side;
                let start = row * side * stride + column * side;
                // RFC 6386 section 15.1 fixes this order because edges share pixels.
                if column != 0 {
                    filter_edge(plane, start, stride, 1, side, limits, true);
                }
                if !block.skip || block.b_pred {
                    for offset in (4..side).step_by(4) {
                        filter_edge(plane, start + offset, stride, 1, side, limits, false);
                    }
                }
                if row != 0 {
                    filter_edge(plane, start, 1, stride, side, limits, true);
                }
                if !block.skip || block.b_pred {
                    for offset in (4..side).step_by(4) {
                        filter_edge(
                            plane,
                            start + offset * stride,
                            1,
                            stride,
                            side,
                            limits,
                            false,
                        );
                    }
                }
            }
        }
    }
}

fn filter_edge(
    plane: &mut [u8],
    start: usize,
    along: usize,
    across: usize,
    length: usize,
    limits: Limits,
    macroblock: bool,
) {
    for position in 0..length {
        let first = start + position * along - 4 * across;
        let mut segment = core::array::from_fn(|i| plane[first + i * across]);
        filter_segment(&mut segment, limits, macroblock);
        for (i, value) in segment.into_iter().enumerate() {
            plane[first + i * across] = value;
        }
    }
}

fn filter_segment(segment: &mut [u8; 8], limits: Limits, macroblock: bool) {
    let mut signed = segment.map(|value| i32::from(value) - 128);
    let edge_limit = if macroblock {
        limits.macroblock
    } else {
        limits.subblock
    };
    let edge_difference = 2 * (signed[3] - signed[4]).abs() + (signed[2] - signed[5]).abs() / 2;
    // RFC 6386 section 15.3 preserves edges that exceed either limit.
    if edge_difference > edge_limit
        || [0, 1, 2, 4, 5, 6]
            .into_iter()
            .any(|i| (signed[i] - signed[i + 1]).abs() > limits.interior)
    {
        return;
    }
    let high_variance = (signed[2] - signed[3]).abs() > limits.variance
        || (signed[4] - signed[5]).abs() > limits.variance;
    if macroblock && !high_variance {
        let difference = (signed[2] - signed[5]).clamp(-128, 127);
        let correction = (difference + 3 * (signed[4] - signed[3])).clamp(-128, 127);
        for (distance, weight) in [27, 18, 9].into_iter().enumerate() {
            let adjustment = ((weight * correction + 63) >> 7).clamp(-128, 127);
            signed[3 - distance] += adjustment;
            signed[4 + distance] -= adjustment;
        }
    } else {
        let outer = if high_variance {
            (signed[2] - signed[5]).clamp(-128, 127)
        } else {
            0
        };
        let correction = (outer + 3 * (signed[4] - signed[3])).clamp(-128, 127);
        // RFC 6386 section 15.2 clamps before each rounded shift.
        let after = (correction + 4).clamp(-128, 127) >> 3;
        let before = (correction + 3).clamp(-128, 127) >> 3;
        signed[3] += before;
        signed[4] -= after;
        if !high_variance {
            let adjustment = (after + 1) >> 1;
            signed[2] += adjustment;
            signed[5] -= adjustment;
        }
    }
    *segment = signed.map(|value| (value.clamp(-128, 127) + 128) as u8);
}

#[cfg(test)]
mod tests {
    use super::{filter_frame, filter_segment, Limits, Macroblock};
    use std::vec::Vec;

    #[test]
    fn level_twenty_six_sets_the_key_frame_limits() {
        assert_eq!(
            Limits::new(26, 0),
            Limits {
                interior: 26,
                macroblock: 82,
                subblock: 78,
                variance: 1
            }
        );
    }

    #[test]
    fn sharpness_three_caps_the_interior_limit_at_six() {
        assert_eq!(
            Limits::new(40, 3),
            Limits {
                interior: 6,
                macroblock: 90,
                subblock: 86,
                variance: 2
            }
        );
    }

    #[test]
    fn sharpness_seven_caps_the_interior_limit_at_two() {
        assert_eq!(
            Limits::new(63, 7),
            Limits {
                interior: 2,
                macroblock: 132,
                subblock: 128,
                variance: 2
            }
        );
    }

    #[test]
    fn level_five_uses_a_zero_variance_threshold() {
        assert_eq!(
            Limits::new(5, 0),
            Limits {
                interior: 5,
                macroblock: 19,
                subblock: 15,
                variance: 0
            }
        );
    }

    #[test]
    fn macroblock_edges_with_high_variance_change_only_the_nearest_pair() {
        let mut segment = [60, 62, 64, 66, 80, 82, 84, 86];
        filter_segment(&mut segment, Limits::new(26, 0), true);
        assert_eq!(segment, [60, 62, 64, 69, 77, 82, 84, 86]);
    }

    #[test]
    fn subblock_edges_with_high_variance_change_only_the_nearest_pair() {
        let mut segment = [60, 62, 64, 66, 80, 82, 84, 86];
        filter_segment(&mut segment, Limits::new(26, 0), false);
        assert_eq!(segment, [60, 62, 64, 69, 77, 82, 84, 86]);
    }

    #[test]
    fn macroblock_edges_spread_a_twenty_step_across_six_pixels() {
        let mut segment = [100, 100, 100, 100, 120, 120, 120, 120];
        filter_segment(&mut segment, Limits::new(40, 0), true);
        assert_eq!(segment, [100, 103, 106, 108, 112, 114, 117, 120]);
    }

    #[test]
    fn subblock_edges_spread_a_twenty_step_across_four_pixels() {
        let mut segment = [100, 100, 100, 100, 120, 120, 120, 120];
        filter_segment(&mut segment, Limits::new(40, 0), false);
        assert_eq!(segment, [100, 100, 104, 107, 112, 116, 120, 120]);
    }

    #[test]
    fn level_twenty_smooths_a_low_variance_macroblock_edge() {
        let mut segment = [128, 128, 130, 131, 140, 141, 141, 141];
        filter_segment(&mut segment, Limits::new(20, 0), true);
        assert_eq!(segment, [128, 129, 132, 134, 137, 139, 140, 141]);
    }

    #[test]
    fn level_zero_leaves_every_plane_unchanged() {
        let source: [Vec<u8>; 3] = core::array::from_fn(|plane| {
            let side = if plane == 0 { 16 } else { 8 };
            (0..4 * side * side)
                .map(|i| ((i * 37 + plane * 71) % 256) as u8)
                .collect()
        });
        for sharpness in 0..=7 {
            let [mut y, mut u, mut v] = source.clone();
            let blocks = [Macroblock {
                skip: false,
                b_pred: true,
            }; 4];
            filter_frame([&mut y, &mut u, &mut v], 2, 2, 0, sharpness, &blocks);
            assert_eq!([y, u, v], source);
        }
    }

    fn step_plane(side: usize, vertical: bool, before: u8, after: u8) -> Vec<u8> {
        let width = if vertical { side * 2 } else { side };
        let height = if vertical { side } else { side * 2 };
        (0..width * height)
            .map(|i| {
                let position = if vertical { i % width } else { i / width };
                if position < side {
                    before
                } else {
                    after
                }
            })
            .collect()
    }

    #[test]
    fn two_macroblocks_change_exactly_six_pixels_across_each_shared_edge() {
        for vertical in [true, false] {
            let columns = if vertical { 2 } else { 1 };
            let rows = if vertical { 1 } else { 2 };
            let source = [
                step_plane(16, vertical, 100, 120),
                step_plane(8, vertical, 100, 120),
                step_plane(8, vertical, 100, 120),
            ];
            let [mut y, mut u, mut v] = source.clone();
            let blocks = [Macroblock {
                skip: true,
                b_pred: false,
            }; 2];
            filter_frame([&mut y, &mut u, &mut v], columns, rows, 40, 0, &blocks);
            for ((actual, initial), side) in [y, u, v].into_iter().zip(source).zip([16, 8, 8]) {
                let mut expected = initial.clone();
                let width = columns * side;
                for along in 0..side {
                    for (offset, value) in [103, 106, 108, 112, 114, 117].into_iter().enumerate() {
                        let position = side - 3 + offset;
                        let index = if vertical {
                            along * width + position
                        } else {
                            position * width + along
                        };
                        expected[index] = value;
                    }
                }
                assert_eq!(
                    actual.iter().zip(&initial).filter(|(a, b)| a != b).count(),
                    6 * side
                );
                assert_eq!(actual, expected);
            }
        }
    }

    #[test]
    fn skipped_macroblocks_filter_subblocks_only_when_they_use_subblock_prediction() {
        for vertical in [true, false] {
            for skip in [true, false] {
                for b_pred in [true, false] {
                    let mut y = step_plane(8, true, 100, 120);
                    y.extend_from_within(..);
                    let mut chroma = step_plane(4, true, 100, 120);
                    chroma.extend_from_within(..);
                    if !vertical {
                        y = transpose(&y, 16);
                        chroma = transpose(&chroma, 8);
                    }
                    let mut expected_y = y.clone();
                    let mut expected_chroma = chroma.clone();
                    if !skip || b_pred {
                        for (expected, side) in [(&mut expected_y, 16), (&mut expected_chroma, 8)] {
                            for along in 0..side {
                                for (offset, value) in [104, 107, 112, 116].into_iter().enumerate()
                                {
                                    let across = side / 2 - 2 + offset;
                                    let index = if vertical {
                                        along * side + across
                                    } else {
                                        across * side + along
                                    };
                                    expected[index] = value;
                                }
                            }
                        }
                    }
                    let mut u = chroma.clone();
                    let mut v = chroma;
                    filter_frame(
                        [&mut y, &mut u, &mut v],
                        1,
                        1,
                        40,
                        3,
                        &[Macroblock { skip, b_pred }],
                    );
                    assert_eq!(y, expected_y);
                    assert_eq!(u, expected_chroma);
                    assert_eq!(v, expected_chroma);
                }
            }
        }
    }

    fn transpose(plane: &[u8], side: usize) -> Vec<u8> {
        (0..plane.len())
            .map(|i| plane[(i % side) * side + i / side])
            .collect()
    }

    #[test]
    fn an_excessive_interior_difference_preserves_the_segment() {
        for index in [0, 1, 2, 5, 6, 7] {
            let mut segment = [100; 8];
            segment[index] = 107;
            let expected = segment;
            for macroblock in [true, false] {
                filter_segment(&mut segment, Limits::new(40, 3), macroblock);
                assert_eq!(segment, expected);
            }
        }
    }

    #[test]
    fn an_excessive_edge_difference_preserves_the_segment() {
        let expected = [100, 100, 100, 100, 120, 120, 120, 120];
        for macroblock in [true, false] {
            let mut segment = expected;
            filter_segment(&mut segment, Limits::new(5, 0), macroblock);
            assert_eq!(segment, expected);
        }
    }

    #[test]
    fn a_negative_step_keeps_the_signed_rounding() {
        let mut segment = [120, 120, 120, 120, 100, 100, 100, 100];
        filter_segment(&mut segment, Limits::new(40, 0), false);
        assert_eq!(segment, [120, 120, 117, 112, 107, 103, 100, 100]);
    }

    #[test]
    fn sharpness_keeps_the_interior_limit_at_least_one() {
        assert_eq!(
            Limits::new(1, 7),
            Limits {
                interior: 1,
                macroblock: 7,
                subblock: 3,
                variance: 0
            }
        );
    }
}
