#![cfg_attr(not(test), allow(dead_code))]

#[cfg(test)]
extern crate std;

// RFC 6386 section 14.1 defines the DC dequantization factors.
pub(crate) const DC_QLOOKUP: [i32; 128] = [
    4, 5, 6, 7, 8, 9, 10, 10, 11, 12, 13, 14, 15, 16, 17, 17, 18, 19, 20, 20, 21, 21, 22, 22, 23,
    23, 24, 25, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 37, 38, 39, 40, 41, 42, 43, 44,
    45, 46, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67,
    68, 69, 70, 71, 72, 73, 74, 75, 76, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 91,
    93, 95, 96, 98, 100, 101, 102, 104, 106, 108, 110, 112, 114, 116, 118, 122, 124, 126, 128, 130,
    132, 134, 136, 138, 140, 143, 145, 148, 151, 154, 157,
];

// RFC 6386 section 14.1 defines the AC dequantization factors.
pub(crate) const AC_QLOOKUP: [i32; 128] = [
    4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28,
    29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52,
    53, 54, 55, 56, 57, 58, 60, 62, 64, 66, 68, 70, 72, 74, 76, 78, 80, 82, 84, 86, 88, 90, 92, 94,
    96, 98, 100, 102, 104, 106, 108, 110, 112, 114, 116, 119, 122, 125, 128, 131, 134, 137, 140,
    143, 146, 149, 152, 155, 158, 161, 164, 167, 170, 173, 177, 181, 185, 189, 193, 197, 201, 205,
    209, 213, 217, 221, 225, 229, 234, 239, 245, 249, 254, 259, 264, 269, 274, 279, 284,
];

// This table stores floor(127 * (1 - cbrt(linear))), where c = q / 100 and
// linear = 2 * c / 3 below 0.75 or 2 * c - 1 otherwise.
pub(crate) const Q_TO_INDEX: [u8; 101] = [
    127, 103, 96, 92, 89, 86, 83, 81, 79, 77, 75, 73, 72, 70, 69, 68, 66, 65, 64, 63, 62, 61, 60,
    59, 58, 57, 56, 55, 54, 53, 52, 51, 51, 50, 49, 48, 48, 47, 46, 45, 45, 44, 43, 43, 42, 41, 41,
    40, 40, 39, 38, 38, 37, 37, 36, 36, 35, 35, 34, 33, 33, 32, 32, 31, 31, 30, 30, 29, 29, 28, 28,
    28, 27, 27, 26, 26, 24, 23, 22, 21, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4,
    3, 2, 1, 0, 0,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct QuantizationFactors {
    pub(crate) y_dc: i32,
    pub(crate) y_ac: i32,
    pub(crate) y2_dc: i32,
    pub(crate) y2_ac: i32,
    pub(crate) chroma_dc: i32,
    pub(crate) chroma_ac: i32,
}

pub(crate) fn quantizer_index(quality: u8) -> u8 {
    Q_TO_INDEX[usize::from(quality.min(100))]
}

pub(crate) fn factors(index: u8) -> QuantizationFactors {
    let index = usize::from(index);
    let dc = DC_QLOOKUP[index];
    let ac = AC_QLOOKUP[index];
    QuantizationFactors {
        y_dc: dc,
        y_ac: ac,
        y2_dc: 2 * dc,
        y2_ac: (ac * 155 / 100).max(8),
        chroma_dc: dc.min(132),
        chroma_ac: ac,
    }
}

pub(crate) fn quantize_block(
    coefficients: &[i32; 16],
    dc_factor: i32,
    ac_factor: i32,
) -> [i16; 16] {
    let mut levels = [0; 16];
    levels[0] = quantize_coefficient(coefficients[0], dc_factor);
    for position in 1..16 {
        levels[position] = quantize_coefficient(coefficients[position], ac_factor);
    }
    levels
}

pub(crate) fn dequantize_block(levels: &[i16; 16], dc_factor: i32, ac_factor: i32) -> [i32; 16] {
    let mut coefficients = [0; 16];
    coefficients[0] = dequantize_coefficient(levels[0], dc_factor);
    for position in 1..16 {
        coefficients[position] = dequantize_coefficient(levels[position], ac_factor);
    }
    coefficients
}

fn quantize_coefficient(coefficient: i32, factor: i32) -> i16 {
    let magnitude = coefficient.abs();
    let level = ((2 * magnitude + factor) / (2 * factor)).min(2047) as i16;
    if coefficient < 0 {
        -level
    } else {
        level
    }
}

fn dequantize_coefficient(level: i16, factor: i32) -> i32 {
    i32::from(level) * factor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_dc_lookup_matches_the_section_14_1_pins() {
        assert_eq!(DC_QLOOKUP.len(), 128);
        assert_eq!(DC_QLOOKUP.first(), Some(&4));
        assert_eq!(DC_QLOOKUP.last(), Some(&157));
        assert_eq!(DC_QLOOKUP.iter().sum::<i32>(), 8168);
    }

    #[test]
    fn the_ac_lookup_matches_the_section_14_1_pins() {
        assert_eq!(AC_QLOOKUP.len(), 128);
        assert_eq!(AC_QLOOKUP.first(), Some(&4));
        assert_eq!(AC_QLOOKUP.last(), Some(&284));
        assert_eq!(AC_QLOOKUP.iter().sum::<i32>(), 12723);
    }

    #[test]
    fn the_quality_lookup_matches_its_pins_and_never_increases() {
        assert_eq!(Q_TO_INDEX.len(), 101);
        assert_eq!(Q_TO_INDEX.first(), Some(&127));
        assert_eq!(Q_TO_INDEX.last(), Some(&0));
        assert_eq!(
            Q_TO_INDEX
                .iter()
                .map(|value| u32::from(*value))
                .sum::<u32>(),
            4184
        );
        assert_eq!(
            [0usize, 25, 50, 75, 90, 95, 100].map(|quality| Q_TO_INDEX[quality]),
            [127, 57, 38, 26, 9, 4, 0]
        );
        assert_eq!(
            Q_TO_INDEX
                .windows(2)
                .filter(|pair| pair[0] < pair[1])
                .count(),
            0
        );
        assert_eq!(quantizer_index(255), quantizer_index(100));
    }

    #[test]
    fn each_plane_uses_the_six_ruled_factors() {
        assert_eq!(
            factors(0),
            QuantizationFactors {
                y_dc: 4,
                y_ac: 4,
                y2_dc: 8,
                y2_ac: 8,
                chroma_dc: 4,
                chroma_ac: 4,
            }
        );
        assert_eq!(
            factors(127),
            QuantizationFactors {
                y_dc: 157,
                y_ac: 284,
                y2_dc: 314,
                y2_ac: 440,
                chroma_dc: 132,
                chroma_ac: 284,
            }
        );
    }

    #[test]
    fn quantizing_and_dequantizing_stays_within_half_a_factor() {
        for index in 0..128u8 {
            let set = factors(index);
            for (factor, bound) in [
                (set.y_dc, 2040),
                (set.y_ac, 2040),
                (set.y2_dc, 16320),
                (set.y2_ac, 16320),
                (set.chroma_dc, 2040),
                (set.chroma_ac, 2040),
            ] {
                for coefficient in -bound..=bound {
                    let level = quantize_coefficient(coefficient, factor);
                    let restored = dequantize_coefficient(level, factor);
                    let excess = ((restored - coefficient).abs() - factor / 2).max(0);
                    assert_eq!(
                        excess, 0,
                        "index {index}, factor {factor}, coefficient {coefficient}"
                    );
                }
            }
        }
    }

    #[test]
    fn block_quantization_keeps_raster_positions_and_factor_classes() {
        let coefficients = [
            20, -20, 30, -30, 40, -40, 50, -50, 60, -60, 70, -70, 80, -80, 90, -90,
        ];
        let levels = quantize_block(&coefficients, 4, 10);
        assert_eq!(
            levels,
            [5, -2, 3, -3, 4, -4, 5, -5, 6, -6, 7, -7, 8, -8, 9, -9]
        );
        assert_eq!(
            dequantize_block(&levels, 4, 10),
            [20, -20, 30, -30, 40, -40, 50, -50, 60, -60, 70, -70, 80, -80, 90, -90]
        );
    }

    #[test]
    fn the_quantizer_saturates_each_level_at_2047() {
        let coefficients = [100_000, -100_000, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(
            quantize_block(&coefficients, 4, 4),
            [2047, -2047, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn extreme_residual_blocks_stay_below_the_level_limit_at_index_zero() {
        let set = factors(0);
        let largest_levels = [
            quantize_block(&[2040; 16], set.y_dc, set.y_dc)[0],
            quantize_block(&[2040; 16], set.y_ac, set.y_ac)[0],
            quantize_block(&[16320; 16], set.y2_dc, set.y2_dc)[0],
            quantize_block(&[16320; 16], set.y2_ac, set.y2_ac)[0],
            quantize_block(&[2040; 16], set.chroma_dc, set.chroma_dc)[0],
            quantize_block(&[2040; 16], set.chroma_ac, set.chroma_ac)[0],
        ];
        assert_eq!(largest_levels, [510, 510, 2040, 2040, 510, 510]);
    }
}
