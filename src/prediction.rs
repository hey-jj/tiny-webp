const LUMA_SIDE: usize = 16;
const CHROMA_SIDE: usize = 8;

pub(crate) fn predict_luma_dc(
    reconstruction: &[u8],
    stride: usize,
    macroblock_x: usize,
    macroblock_y: usize,
) -> [u8; LUMA_SIDE * LUMA_SIDE] {
    [dc_value::<LUMA_SIDE, 4, 5>(reconstruction, stride, macroblock_x, macroblock_y);
        LUMA_SIDE * LUMA_SIDE]
}

pub(crate) fn predict_chroma_dc(
    reconstruction: &[u8],
    stride: usize,
    macroblock_x: usize,
    macroblock_y: usize,
) -> [u8; CHROMA_SIDE * CHROMA_SIDE] {
    [dc_value::<CHROMA_SIDE, 3, 4>(reconstruction, stride, macroblock_x, macroblock_y);
        CHROMA_SIDE * CHROMA_SIDE]
}

fn dc_value<const SIDE: usize, const EDGE_SHIFT: u32, const BOTH_SHIFT: u32>(
    reconstruction: &[u8],
    stride: usize,
    macroblock_x: usize,
    macroblock_y: usize,
) -> u8 {
    if macroblock_x == 0 && macroblock_y == 0 {
        return 128;
    }

    let x = macroblock_x * SIDE;
    let y = macroblock_y * SIDE;
    let mut sum = 0i32;

    if macroblock_y > 0 {
        let above = (y - 1) * stride + x;
        for column in 0..SIDE {
            sum += i32::from(reconstruction[above + column]);
        }
    }

    if macroblock_x > 0 {
        let left = x - 1;
        for row in 0..SIDE {
            sum += i32::from(reconstruction[(y + row) * stride + left]);
        }
    }

    let shift = if macroblock_x > 0 && macroblock_y > 0 {
        BOTH_SHIFT
    } else {
        EDGE_SHIFT
    };
    ((sum + (1 << (shift - 1))) >> shift) as u8
}

#[cfg(test)]
mod tests {
    use super::{predict_chroma_dc, predict_luma_dc};
    use std::vec;

    #[test]
    fn the_top_left_macroblock_predicts_one_hundred_twenty_eight_for_luma_and_chroma() {
        let luma = vec![19; 16 * 16];
        let chroma = vec![37; 8 * 8];

        assert_eq!(predict_luma_dc(&luma, 16, 0, 0), [128; 16 * 16]);
        assert_eq!(predict_chroma_dc(&chroma, 8, 0, 0), [128; 8 * 8]);
    }

    #[test]
    fn a_top_row_macroblock_rounds_the_average_of_the_left_column() {
        let mut luma = vec![0; 32 * 16];
        for row in 0..16 {
            luma[row * 32 + 15] = if row < 8 { 10 } else { 11 };
        }
        let mut chroma = vec![0; 16 * 8];
        for row in 0..8 {
            chroma[row * 16 + 7] = if row < 4 { 20 } else { 21 };
        }

        assert_eq!(predict_luma_dc(&luma, 32, 1, 0), [11; 16 * 16]);
        assert_eq!(predict_chroma_dc(&chroma, 16, 1, 0), [21; 8 * 8]);
    }

    #[test]
    fn a_left_column_macroblock_rounds_the_average_of_the_row_above() {
        let mut luma = vec![0; 16 * 32];
        for column in 0..16 {
            luma[15 * 16 + column] = if column < 8 { 30 } else { 31 };
        }
        let mut chroma = vec![0; 8 * 16];
        for column in 0..8 {
            chroma[7 * 8 + column] = if column < 4 { 40 } else { 41 };
        }

        assert_eq!(predict_luma_dc(&luma, 16, 0, 1), [31; 16 * 16]);
        assert_eq!(predict_chroma_dc(&chroma, 8, 0, 1), [41; 8 * 8]);
    }

    #[test]
    fn an_inner_macroblock_rounds_the_average_of_both_borders() {
        let mut luma = vec![0; 32 * 32];
        for column in 16..32 {
            luma[15 * 32 + column] = 60;
        }
        for row in 16..32 {
            luma[row * 32 + 15] = 61;
        }
        let mut chroma = vec![0; 16 * 16];
        for column in 8..16 {
            chroma[7 * 16 + column] = 70;
        }
        for row in 8..16 {
            chroma[row * 16 + 7] = 71;
        }

        assert_eq!(predict_luma_dc(&luma, 32, 1, 1), [61; 16 * 16]);
        assert_eq!(predict_chroma_dc(&chroma, 16, 1, 1), [71; 8 * 8]);
    }
}
