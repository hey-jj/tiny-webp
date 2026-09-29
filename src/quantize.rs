#[cfg(test)]
extern crate std;

// BIT_COST[p] = round(-256 * log2(p / 256)) for p in 1 through 255.
// Index 0 is unused and holds 2048.
pub(crate) const BIT_COST: [u16; 256] = [
    2048, 2048, 1792, 1642, 1536, 1454, 1386, 1329, 1280, 1236, 1198, 1162, 1130, 1101, 1073, 1048,
    1024, 1002, 980, 961, 942, 924, 906, 890, 874, 859, 845, 831, 817, 804, 792, 780, 768, 757,
    746, 735, 724, 714, 705, 695, 686, 676, 668, 659, 650, 642, 634, 626, 618, 611, 603, 596, 589,
    582, 575, 568, 561, 555, 548, 542, 536, 530, 524, 518, 512, 506, 501, 495, 490, 484, 479, 474,
    468, 463, 458, 453, 449, 444, 439, 434, 430, 425, 420, 416, 412, 407, 403, 399, 394, 390, 386,
    382, 378, 374, 370, 366, 362, 358, 355, 351, 347, 343, 340, 336, 333, 329, 326, 322, 319, 315,
    312, 309, 305, 302, 299, 296, 292, 289, 286, 283, 280, 277, 274, 271, 268, 265, 262, 259, 256,
    253, 250, 247, 245, 242, 239, 236, 234, 231, 228, 226, 223, 220, 218, 215, 212, 210, 207, 205,
    202, 200, 197, 195, 193, 190, 188, 185, 183, 181, 178, 176, 174, 171, 169, 167, 164, 162, 160,
    158, 156, 153, 151, 149, 147, 145, 143, 140, 138, 136, 134, 132, 130, 128, 126, 124, 122, 120,
    118, 116, 114, 112, 110, 108, 106, 104, 102, 101, 99, 97, 95, 93, 91, 89, 87, 86, 84, 82, 80,
    78, 77, 75, 73, 71, 70, 68, 66, 64, 63, 61, 59, 58, 56, 54, 53, 51, 49, 48, 46, 44, 43, 41, 40,
    38, 36, 35, 33, 32, 30, 28, 27, 25, 24, 22, 21, 19, 18, 16, 15, 13, 12, 10, 9, 7, 6, 4, 3, 1,
];

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

// The table stores floor(127 * (1 - cbrt(linear))).
// The value c equals q / 100.
// The linear value equals 2 * c / 3 below 0.75.
// The linear value equals 2 * c - 1 at 0.75 and above.
// All 101 entries match the y_ac_qi field written by cwebp 1.6.0.
// The cwebp check uses one segment and disables spatial noise shaping.
pub(crate) const Q_TO_INDEX: [u8; 101] = [
    127, 103, 96, 92, 89, 86, 83, 81, 79, 77, 75, 73, 72, 70, 69, 68, 66, 65, 64, 63, 62, 61, 60,
    59, 58, 57, 56, 55, 54, 53, 52, 51, 51, 50, 49, 48, 48, 47, 46, 45, 45, 44, 43, 43, 42, 41, 41,
    40, 40, 39, 38, 38, 37, 37, 36, 36, 35, 35, 34, 33, 33, 32, 32, 31, 31, 30, 30, 29, 29, 28, 28,
    28, 27, 27, 26, 26, 24, 23, 22, 21, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4,
    3, 2, 1, 0, 0,
];

// RFC 6386 section 9.6 orders Y DC, Y2 DC, Y2 AC, chroma DC, and chroma AC deltas.
pub(crate) const QUANTIZER_DELTAS: [i8; 5] = [0, 0, 0, -2, -4];

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
        chroma_dc: DC_QLOOKUP[index.saturating_sub(QUANTIZER_DELTAS[3].unsigned_abs() as usize)]
            .min(132),
        chroma_ac: AC_QLOOKUP[index.saturating_sub(QUANTIZER_DELTAS[4].unsigned_abs() as usize)],
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
    fn each_plane_uses_its_six_quantization_factors() {
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
                chroma_ac: 264,
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
    #[test]
    fn the_quantizer_deltas_keep_luma_and_reduce_chroma_indexes() {
        assert_eq!(QUANTIZER_DELTAS.len(), 5);
        assert_eq!(QUANTIZER_DELTAS.first(), Some(&0));
        assert_eq!(QUANTIZER_DELTAS.last(), Some(&-4));
        assert_eq!(QUANTIZER_DELTAS.into_iter().map(i32::from).sum::<i32>(), -6);
        assert_eq!(factors(1).chroma_dc, 4);
        assert_eq!(factors(3).chroma_ac, 4);
        assert_eq!(factors(26).chroma_dc, 23);
        assert_eq!(factors(26).chroma_ac, 26);
    }
}
