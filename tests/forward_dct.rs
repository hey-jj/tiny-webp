//! The fixed-point forward DCT against its real-valued definition.

#[path = "../src/transform.rs"]
mod transform;

use std::f64::consts::{FRAC_1_SQRT_2, PI};

fn real_forward_dct(input: &[i32; 16]) -> [i32; 16] {
    let mut output = [0; 16];
    for vertical in 0..4 {
        for horizontal in 0..4 {
            let vertical_scale = if vertical == 0 { FRAC_1_SQRT_2 } else { 1.0 };
            let horizontal_scale = if horizontal == 0 { FRAC_1_SQRT_2 } else { 1.0 };
            let mut sum = 0.0;
            for row in 0..4 {
                for column in 0..4 {
                    let vertical_angle = (2 * row + 1) as f64 * vertical as f64 * PI / 8.0;
                    let horizontal_angle = (2 * column + 1) as f64 * horizontal as f64 * PI / 8.0;
                    sum += f64::from(input[row * 4 + column])
                        * vertical_angle.cos()
                        * horizontal_angle.cos();
                }
            }
            output[vertical * 4 + horizontal] =
                (sum * vertical_scale * horizontal_scale).round() as i32;
        }
    }
    output
}

#[test]
fn the_integer_forward_dct_stays_within_one_of_the_real_transform() {
    let mut state = 0xd1b5_4a32u32;
    let mut largest_difference = 0;
    for _ in 0..4096 {
        let mut block = [0; 16];
        for value in &mut block {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            *value = (state % 511) as i32 - 255;
        }

        let integer = transform::forward_dct(&block);
        let real = real_forward_dct(&block);
        for index in 0..16 {
            let difference = (integer[index] - real[index]).abs();
            largest_difference = largest_difference.max(difference);
        }
    }
    assert_eq!(largest_difference, 1);
}
