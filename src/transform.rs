// Each entry is round(65536 * scale * cos((2 * x + 1) * u * pi / 8)).
// Scale is one half for u = 0 and one over square root of two otherwise.
pub(crate) const DCT_BASIS: [[i32; 4]; 4] = [
    [32768, 32768, 32768, 32768],
    [42813, 17734, -17734, -42813],
    [32768, -32768, -32768, 32768],
    [17734, -42813, 42813, -17734],
];

// RFC 6386 section 14.4 defines these 16.16 inverse DCT constants.
pub(crate) const COS_PI_8_SQRT_2_MINUS_1: i32 = 20091;
pub(crate) const SIN_PI_8_SQRT_2: i32 = 35468;

pub(crate) fn inverse_wht(input: &[i32; 16]) -> [i32; 16] {
    let mut output = [0; 16];

    // RFC 6386 section 14.3 fixes the pass order and the final rounding.
    for column in 0..4 {
        let a = input[column] + input[12 + column];
        let b = input[4 + column] + input[8 + column];
        let c = input[4 + column] - input[8 + column];
        let d = input[column] - input[12 + column];

        output[column] = a + b;
        output[4 + column] = c + d;
        output[8 + column] = a - b;
        output[12 + column] = d - c;
    }

    for row in 0..4 {
        let offset = row * 4;
        let a = output[offset] + output[offset + 3];
        let b = output[offset + 1] + output[offset + 2];
        let c = output[offset + 1] - output[offset + 2];
        let d = output[offset] - output[offset + 3];

        output[offset] = (a + b + 3) >> 3;
        output[offset + 1] = (c + d + 3) >> 3;
        output[offset + 2] = (a - b + 3) >> 3;
        output[offset + 3] = (d - c + 3) >> 3;
    }

    output
}

pub(crate) fn inverse_dct(input: &[i32; 16]) -> [i32; 16] {
    let mut output = [0; 16];

    // RFC 6386 section 14.4 fixes both constants and the final rounding.
    for column in 0..4 {
        let a = input[column] + input[8 + column];
        let b = input[column] - input[8 + column];
        let c = ((input[4 + column] * SIN_PI_8_SQRT_2) >> 16)
            - input[12 + column]
            - ((input[12 + column] * COS_PI_8_SQRT_2_MINUS_1) >> 16);
        let d = input[4 + column]
            + ((input[4 + column] * COS_PI_8_SQRT_2_MINUS_1) >> 16)
            + ((input[12 + column] * SIN_PI_8_SQRT_2) >> 16);

        output[column] = a + d;
        output[4 + column] = b + c;
        output[8 + column] = b - c;
        output[12 + column] = a - d;
    }

    for row in 0..4 {
        let offset = row * 4;
        let a = output[offset] + output[offset + 2];
        let b = output[offset] - output[offset + 2];
        let c = ((output[offset + 1] * SIN_PI_8_SQRT_2) >> 16)
            - output[offset + 3]
            - ((output[offset + 3] * COS_PI_8_SQRT_2_MINUS_1) >> 16);
        let d = output[offset + 1]
            + ((output[offset + 1] * COS_PI_8_SQRT_2_MINUS_1) >> 16)
            + ((output[offset + 3] * SIN_PI_8_SQRT_2) >> 16);

        output[offset] = (a + d + 4) >> 3;
        output[offset + 1] = (b + c + 4) >> 3;
        output[offset + 2] = (b - c + 4) >> 3;
        output[offset + 3] = (a - d + 4) >> 3;
    }

    output
}

pub(crate) fn forward_wht(input: &[i32; 16]) -> [i32; 16] {
    debug_assert!(input.iter().all(|value| (-2040..=2040).contains(value)));

    let mut intermediate = [0; 16];
    for column in 0..4 {
        let values = [
            input[column],
            input[4 + column],
            input[8 + column],
            input[12 + column],
        ];
        let transformed = wht_pass(values);
        for row in 0..4 {
            intermediate[row * 4 + column] = transformed[row];
        }
    }

    let mut output = [0; 16];
    for row in 0..4 {
        let offset = row * 4;
        let transformed = wht_pass([
            intermediate[offset],
            intermediate[offset + 1],
            intermediate[offset + 2],
            intermediate[offset + 3],
        ]);
        for column in 0..4 {
            output[offset + column] = divide_by_two(transformed[column]);
        }
    }

    output
}

pub(crate) fn forward_dct(input: &[i32; 16]) -> [i32; 16] {
    debug_assert!(input.iter().all(|value| (-255..=255).contains(value)));

    let mut output = [0; 16];
    for vertical in 0..4 {
        for horizontal in 0..4 {
            let mut sum = 0i64;
            for row in 0..4 {
                for column in 0..4 {
                    sum += i64::from(input[row * 4 + column])
                        * i64::from(DCT_BASIS[vertical][row])
                        * i64::from(DCT_BASIS[horizontal][column]);
                }
            }
            output[vertical * 4 + horizontal] = rounded_shift(sum * 2, 32);
        }
    }
    output
}

pub(crate) fn clamped_add(prediction: u8, residual: i32) -> u8 {
    // RFC 6386 section 14.5 requires saturation after the 32-bit sum.
    (i32::from(prediction) + residual).clamp(0, 255) as u8
}

fn wht_pass(values: [i32; 4]) -> [i32; 4] {
    let a = values[0] + values[3];
    let b = values[1] + values[2];
    let c = values[1] - values[2];
    let d = values[0] - values[3];
    [a + b, c + d, a - b, d - c]
}

fn divide_by_two(value: i32) -> i32 {
    if value < 0 {
        -((-value + 1) >> 1)
    } else {
        (value + 1) >> 1
    }
}

fn rounded_shift(value: i64, bits: u32) -> i32 {
    let half = 1i64 << (bits - 1);
    if value < 0 {
        -(((-value + half) >> bits) as i32)
    } else {
        ((value + half) >> bits) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::{clamped_add, forward_dct, forward_wht, inverse_dct, inverse_wht};

    #[test]
    fn inverse_dct_spreads_a_lone_dc_value_across_the_block() {
        let mut input = [0; 16];
        input[0] = 800;
        assert_eq!(inverse_dct(&input), [100; 16]);
    }

    #[test]
    fn inverse_dct_spreads_the_first_horizontal_frequency_across_each_row() {
        let mut input = [0; 16];
        input[1] = 100;
        assert_eq!(
            inverse_dct(&input),
            [16, 7, -7, -16, 16, 7, -7, -16, 16, 7, -7, -16, 16, 7, -7, -16]
        );
    }

    #[test]
    fn inverse_dct_spreads_the_first_vertical_frequency_down_each_column() {
        let mut input = [0; 16];
        input[4] = 100;
        assert_eq!(
            inverse_dct(&input),
            [16, 16, 16, 16, 7, 7, 7, 7, -7, -7, -7, -7, -16, -16, -16, -16]
        );
    }

    #[test]
    fn inverse_dct_matches_the_mixed_frequency_vector() {
        let mut input = [0; 16];
        input[0] = -13;
        input[5] = 77;
        input[15] = -1000;
        assert_eq!(
            inverse_dct(&input),
            [-22, 94, -97, 19, 93, -212, 209, -97, -97, 209, -212, 94, 19, -97, 94, -22]
        );
    }

    #[test]
    fn inverse_wht_spreads_a_lone_dc_value_across_the_block() {
        let mut input = [0; 16];
        input[0] = 800;
        assert_eq!(inverse_wht(&input), [100; 16]);
    }

    #[test]
    fn inverse_wht_rounds_a_small_lone_dc_value_across_the_block() {
        let mut input = [0; 16];
        input[0] = 5;
        assert_eq!(inverse_wht(&input), [1; 16]);
    }

    #[test]
    fn inverse_wht_matches_the_mixed_frequency_vector() {
        let mut input = [0; 16];
        input[0] = -13;
        input[1] = 77;
        input[4] = -1000;
        input[15] = 5;
        assert_eq!(
            inverse_wht(&input),
            [
                -116, -118, -136, -137, -118, -116, -137, -136, 134, 132, 114, 113, 132, 134, 113,
                114
            ]
        );
    }

    #[test]
    fn zero_blocks_map_to_zero_coefficients_in_both_forward_transforms() {
        assert_eq!(
            (forward_dct(&[0; 16]), forward_wht(&[0; 16])),
            ([0; 16], [0; 16])
        );
    }

    #[test]
    fn constant_blocks_map_to_one_scaled_dc_coefficient() {
        for value in -255..=255 {
            let mut expected = [0; 16];
            expected[0] = 8 * value;
            assert_eq!(forward_dct(&[value; 16]), expected, "DCT value {value}");
            assert_eq!(forward_wht(&[value; 16]), expected, "WHT value {value}");
        }
    }

    #[test]
    fn inverse_transforms_recover_every_seeded_block_within_one() {
        let mut state = 0x7f4a_7c15u32;
        let mut largest_dct_error = 0;
        let mut largest_wht_error = 0;
        for _ in 0..4096 {
            let mut block = [0; 16];
            for value in &mut block {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                *value = (state % 511) as i32 - 255;
            }

            let dct_round_trip = inverse_dct(&forward_dct(&block));
            let wht_round_trip = inverse_wht(&forward_wht(&block));
            for index in 0..16 {
                let dct_error = (block[index] - dct_round_trip[index]).abs();
                let wht_error = (block[index] - wht_round_trip[index]).abs();
                largest_dct_error = largest_dct_error.max(dct_error);
                largest_wht_error = largest_wht_error.max(wht_error);
            }
        }
        assert_eq!((largest_dct_error, largest_wht_error), (1, 1));
    }

    #[test]
    fn forward_transform_coefficients_reach_the_stated_bounds() {
        let mut largest_dct = 0;
        let mut largest_wht = 0;
        for signs in 0..=u16::MAX {
            let mut dct_input = [0; 16];
            let mut wht_input = [0; 16];
            for index in 0..16 {
                let sign = if signs & (1 << index) == 0 { -1 } else { 1 };
                dct_input[index] = sign * 255;
                wht_input[index] = sign * 2040;
            }
            largest_dct = largest_dct.max(
                forward_dct(&dct_input)
                    .iter()
                    .map(|value| value.abs())
                    .max()
                    .unwrap(),
            );
            largest_wht = largest_wht.max(
                forward_wht(&wht_input)
                    .iter()
                    .map(|value| value.abs())
                    .max()
                    .unwrap(),
            );
        }
        assert_eq!((largest_dct, largest_wht), (2040, 16320));
    }

    #[test]
    fn clamped_add_saturates_both_edges_and_keeps_in_range_sums() {
        assert_eq!(
            (
                clamped_add(4, -9),
                clamped_add(100, 55),
                clamped_add(250, 12)
            ),
            (0, 155, 255)
        );
    }
}
