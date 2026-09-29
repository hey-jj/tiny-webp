const LUMA_SIDE: usize = 16;
const CHROMA_SIDE: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PredictionMode {
    Dc,
    Vertical,
    Horizontal,
    TrueMotion,
    Subblock(SubblockMode),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SubblockMode {
    Dc,
    TrueMotion,
    VerticalEdge,
    HorizontalEdge,
    LeftDown,
    RightDown,
    VerticalRight,
    VerticalLeft,
    HorizontalDown,
    HorizontalUp,
}

pub(crate) struct Candidate {
    pub(crate) mode: PredictionMode,
    pub(crate) enabled: bool,
}

pub(crate) const CANDIDATES: [Candidate; 14] = [
    Candidate {
        mode: PredictionMode::Dc,
        enabled: true,
    },
    Candidate {
        mode: PredictionMode::Vertical,
        enabled: false,
    },
    Candidate {
        mode: PredictionMode::Horizontal,
        enabled: false,
    },
    Candidate {
        mode: PredictionMode::TrueMotion,
        enabled: false,
    },
    Candidate {
        mode: PredictionMode::Subblock(SubblockMode::Dc),
        enabled: false,
    },
    Candidate {
        mode: PredictionMode::Subblock(SubblockMode::TrueMotion),
        enabled: false,
    },
    Candidate {
        mode: PredictionMode::Subblock(SubblockMode::VerticalEdge),
        enabled: false,
    },
    Candidate {
        mode: PredictionMode::Subblock(SubblockMode::HorizontalEdge),
        enabled: false,
    },
    Candidate {
        mode: PredictionMode::Subblock(SubblockMode::LeftDown),
        enabled: false,
    },
    Candidate {
        mode: PredictionMode::Subblock(SubblockMode::RightDown),
        enabled: false,
    },
    Candidate {
        mode: PredictionMode::Subblock(SubblockMode::VerticalRight),
        enabled: false,
    },
    Candidate {
        mode: PredictionMode::Subblock(SubblockMode::VerticalLeft),
        enabled: false,
    },
    Candidate {
        mode: PredictionMode::Subblock(SubblockMode::HorizontalDown),
        enabled: false,
    },
    Candidate {
        mode: PredictionMode::Subblock(SubblockMode::HorizontalUp),
        enabled: false,
    },
];

impl PredictionMode {
    fn predict<const SIDE: usize>(
        self,
        reconstruction: &[u8],
        stride: usize,
        x: usize,
        y: usize,
        output: &mut [u8],
    ) {
        match self {
            Self::Dc => output.fill(dc_value::<SIDE>(reconstruction, stride, x / SIDE, y / SIDE)),
            Self::Subblock(mode) => {
                let edges = SubblockEdges::read(reconstruction, stride, x, y);
                output.copy_from_slice(&edges.predict(mode));
            }
            _ => {
                let corner = pixel(reconstruction, stride, x.checked_sub(1), y.checked_sub(1));
                for (position, value) in output.iter_mut().enumerate() {
                    let row = position / SIDE;
                    let column = position % SIDE;
                    let above = pixel(reconstruction, stride, Some(x + column), y.checked_sub(1));
                    let left = pixel(reconstruction, stride, x.checked_sub(1), Some(y + row));
                    *value = match self {
                        Self::Vertical => above,
                        Self::Horizontal => left,
                        _ => true_motion(left, above, corner),
                    };
                }
            }
        }
    }
}

fn pixel(reconstruction: &[u8], stride: usize, x: Option<usize>, y: Option<usize>) -> u8 {
    // RFC 6386 section 12 gives the top border precedence at the corner.
    match (x, y) {
        (_, None) => 127,
        (None, Some(_)) => 129,
        (Some(x), Some(y)) => reconstruction[y * stride + x],
    }
}

fn true_motion(left: u8, above: u8, corner: u8) -> u8 {
    (i32::from(left) + i32::from(above) - i32::from(corner)).clamp(0, 255) as u8
}

struct SubblockEdges {
    above: [u8; 8],
    left: [u8; 4],
    corner: u8,
}

impl SubblockEdges {
    fn read(reconstruction: &[u8], stride: usize, x: usize, y: usize) -> Self {
        let mut above = [0; 8];
        for (column, value) in above.iter_mut().enumerate() {
            // RFC 6386 section 12.3 reuses the macroblock's top-right edge.
            *value = if x % 16 == 12 && column >= 4 {
                let macroblock_y = y / 16 * 16;
                let source_x = (x + column).min(stride - 1);
                pixel(
                    reconstruction,
                    stride,
                    Some(source_x),
                    macroblock_y.checked_sub(1),
                )
            } else {
                pixel(reconstruction, stride, Some(x + column), y.checked_sub(1))
            };
        }
        Self {
            above,
            left: core::array::from_fn(|row| {
                pixel(reconstruction, stride, x.checked_sub(1), Some(y + row))
            }),
            corner: pixel(reconstruction, stride, x.checked_sub(1), y.checked_sub(1)),
        }
    }

    fn predict(&self, mode: SubblockMode) -> [u8; 16] {
        let a = self.above;
        let l = self.left;
        let p = self.corner;
        let edge = [l[3], l[2], l[1], l[0], p, a[0], a[1], a[2], a[3]];
        let smooth = |index: usize| average_three(edge[index - 1], edge[index], edge[index + 1]);
        let halfway = |index: usize| average_two(edge[index], edge[index + 1]);
        let dc = ((4 + a[..4]
            .iter()
            .chain(l.iter())
            .map(|&v| u16::from(v))
            .sum::<u16>())
            >> 3) as u8;
        core::array::from_fn(|position| {
            let row = position / 4;
            let column = position % 4;
            match mode {
                SubblockMode::Dc => dc,
                SubblockMode::TrueMotion => true_motion(l[row], a[column], p),
                SubblockMode::VerticalEdge => average_three(
                    if column == 0 { p } else { a[column - 1] },
                    a[column],
                    a[column + 1],
                ),
                SubblockMode::HorizontalEdge => average_three(
                    if row == 0 { p } else { l[row - 1] },
                    l[row],
                    l[(row + 1).min(3)],
                ),
                SubblockMode::LeftDown => {
                    let index = row + column;
                    average_three(a[index], a[index + 1], a[(index + 2).min(7)])
                }
                SubblockMode::RightDown => smooth(4 + column - row),
                SubblockMode::VerticalRight => {
                    let offset = 2 * column as i32 - row as i32;
                    if offset < 0 {
                        smooth((5 + offset) as usize)
                    } else if offset % 2 == 0 {
                        halfway(4 + offset as usize / 2)
                    } else {
                        smooth(5 + offset as usize / 2)
                    }
                }
                SubblockMode::VerticalLeft => {
                    let index = column + row / 2;
                    if column == 3 && row >= 2 {
                        average_three(a[row + 2], a[row + 3], a[row + 4])
                    } else if row % 2 == 0 {
                        average_two(a[index], a[index + 1])
                    } else {
                        average_three(a[index], a[index + 1], a[index + 2])
                    }
                }
                SubblockMode::HorizontalDown => {
                    let offset = 2 * row as i32 - column as i32;
                    if offset < 0 {
                        smooth((3 - offset) as usize)
                    } else if offset % 2 == 0 {
                        halfway(3 - offset as usize / 2)
                    } else {
                        smooth(3 - offset as usize / 2)
                    }
                }
                SubblockMode::HorizontalUp => {
                    let index = row + column / 2;
                    if index >= 3 {
                        l[3]
                    } else if column % 2 == 0 {
                        average_two(l[index], l[index + 1])
                    } else {
                        average_three(l[index], l[index + 1], l[(index + 2).min(3)])
                    }
                }
            }
        })
    }
}

fn average_two(first: u8, second: u8) -> u8 {
    ((u16::from(first) + u16::from(second) + 1) >> 1) as u8
}

fn average_three(first: u8, center: u8, last: u8) -> u8 {
    ((u16::from(first) + 2 * u16::from(center) + u16::from(last) + 2) >> 2) as u8
}

pub(crate) fn predict_luma(
    reconstruction: &[u8],
    stride: usize,
    macroblock_x: usize,
    macroblock_y: usize,
) -> [u8; LUMA_SIDE * LUMA_SIDE] {
    let mut output = [0; LUMA_SIDE * LUMA_SIDE];
    for candidate in CANDIDATES.iter().filter(|candidate| candidate.enabled) {
        candidate.mode.predict::<LUMA_SIDE>(
            reconstruction,
            stride,
            macroblock_x * LUMA_SIDE,
            macroblock_y * LUMA_SIDE,
            &mut output,
        );
    }
    output
}

pub(crate) fn predict_chroma(
    reconstruction: &[u8],
    stride: usize,
    macroblock_x: usize,
    macroblock_y: usize,
) -> [u8; CHROMA_SIDE * CHROMA_SIDE] {
    let mut output = [0; CHROMA_SIDE * CHROMA_SIDE];
    for candidate in CANDIDATES.iter().filter(|candidate| candidate.enabled) {
        candidate.mode.predict::<CHROMA_SIDE>(
            reconstruction,
            stride,
            macroblock_x * CHROMA_SIDE,
            macroblock_y * CHROMA_SIDE,
            &mut output,
        );
    }
    output
}

fn dc_value<const SIDE: usize>(
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
        SIDE.ilog2() + 1
    } else {
        SIDE.ilog2()
    };
    ((sum + (1 << (shift - 1))) >> shift) as u8
}

#[cfg(test)]
mod tests {
    use super::{
        predict_chroma, predict_luma, PredictionMode, SubblockEdges, SubblockMode, CANDIDATES,
    };
    use std::vec;

    #[test]
    fn the_top_left_macroblock_predicts_one_hundred_twenty_eight_for_luma_and_chroma() {
        let luma = vec![19; 16 * 16];
        let chroma = vec![37; 8 * 8];

        assert_eq!(predict_luma(&luma, 16, 0, 0), [128; 16 * 16]);
        assert_eq!(predict_chroma(&chroma, 8, 0, 0), [128; 8 * 8]);
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

        assert_eq!(predict_luma(&luma, 32, 1, 0), [11; 16 * 16]);
        assert_eq!(predict_chroma(&chroma, 16, 1, 0), [21; 8 * 8]);
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

        assert_eq!(predict_luma(&luma, 16, 0, 1), [31; 16 * 16]);
        assert_eq!(predict_chroma(&chroma, 8, 0, 1), [41; 8 * 8]);
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

        assert_eq!(predict_luma(&luma, 32, 1, 1), [61; 16 * 16]);
        assert_eq!(predict_chroma(&chroma, 16, 1, 1), [71; 8 * 8]);
    }

    #[test]
    fn only_the_full_block_dc_candidate_is_enabled() {
        let enabled: std::vec::Vec<_> = CANDIDATES
            .iter()
            .filter(|candidate| candidate.enabled)
            .map(|candidate| candidate.mode)
            .collect();
        assert_eq!(enabled, [PredictionMode::Dc]);
        assert_eq!(CANDIDATES.len(), 14);
    }

    fn pinned_edges() -> SubblockEdges {
        SubblockEdges {
            above: [100, 110, 120, 130, 140, 150, 160, 170],
            left: [80, 70, 60, 50],
            corner: 90,
        }
    }

    #[test]
    fn dc_averages_the_eight_subblock_neighbors() {
        assert_eq!(pinned_edges().predict(SubblockMode::Dc), [90; 16]);
    }

    #[test]
    fn true_motion_preserves_both_subblock_edge_differences() {
        assert_eq!(
            pinned_edges().predict(SubblockMode::TrueMotion),
            [90, 100, 110, 120, 80, 90, 100, 110, 70, 80, 90, 100, 60, 70, 80, 90]
        );
    }

    #[test]
    fn vertical_edge_repeats_the_smoothed_above_row() {
        assert_eq!(
            pinned_edges().predict(SubblockMode::VerticalEdge),
            [100, 110, 120, 130, 100, 110, 120, 130, 100, 110, 120, 130, 100, 110, 120, 130]
        );
    }

    #[test]
    fn horizontal_edge_repeats_the_smoothed_left_column() {
        assert_eq!(
            pinned_edges().predict(SubblockMode::HorizontalEdge),
            [80, 80, 80, 80, 70, 70, 70, 70, 60, 60, 60, 60, 53, 53, 53, 53]
        );
    }

    #[test]
    fn left_down_extends_the_above_row_along_diagonals() {
        assert_eq!(
            pinned_edges().predict(SubblockMode::LeftDown),
            [110, 120, 130, 140, 120, 130, 140, 150, 130, 140, 150, 160, 140, 150, 160, 168]
        );
    }

    #[test]
    fn right_down_extends_both_edges_along_diagonals() {
        assert_eq!(
            pinned_edges().predict(SubblockMode::RightDown),
            [90, 100, 110, 120, 80, 90, 100, 110, 70, 80, 90, 100, 60, 70, 80, 90]
        );
    }

    #[test]
    fn vertical_right_interleaves_half_steps_and_smoothed_edges() {
        assert_eq!(
            pinned_edges().predict(SubblockMode::VerticalRight),
            [95, 105, 115, 125, 90, 100, 110, 120, 80, 95, 105, 115, 70, 90, 100, 110]
        );
    }

    #[test]
    fn vertical_left_uses_three_pixel_averages_at_the_lower_right() {
        assert_eq!(
            pinned_edges().predict(SubblockMode::VerticalLeft),
            [105, 115, 125, 135, 110, 120, 130, 140, 115, 125, 135, 150, 120, 130, 140, 160]
        );
    }

    #[test]
    fn horizontal_down_interleaves_half_steps_from_the_left_edge() {
        assert_eq!(
            pinned_edges().predict(SubblockMode::HorizontalDown),
            [85, 90, 100, 110, 75, 80, 85, 90, 65, 70, 75, 80, 55, 60, 65, 70]
        );
    }

    #[test]
    fn horizontal_up_repeats_the_last_left_pixel_beyond_the_edge() {
        assert_eq!(
            pinned_edges().predict(SubblockMode::HorizontalUp),
            [75, 70, 65, 60, 65, 60, 55, 53, 55, 53, 50, 50, 50, 50, 50, 50]
        );
    }

    fn check_full_block_edges<const SIDE: usize>() {
        let stride = SIDE * 2;
        let mut plane = vec![77; stride * stride];
        for column in 0..stride {
            plane[(SIDE - 1) * stride + column] = 200;
        }
        for row in 0..stride {
            plane[row * stride + SIDE - 1] = 30;
        }
        plane[(SIDE - 1) * stride + SIDE - 1] = 90;
        let mut output = vec![0; SIDE * SIDE];
        for (x, y, expected) in [
            (0, 0, 129),
            (SIDE, 0, 30),
            (0, SIDE, 200),
            (SIDE, SIDE, 140),
        ] {
            PredictionMode::TrueMotion.predict::<SIDE>(&plane, stride, x, y, &mut output);
            let mut expected_block = vec![expected; SIDE * SIDE];
            if x == SIDE && y == 0 {
                expected_block[(SIDE - 1) * SIDE..].fill(90);
            }
            if x == 0 && y == SIDE {
                for row in 0..SIDE {
                    expected_block[row * SIDE + SIDE - 1] = 90;
                }
            }
            assert_eq!(output, expected_block);
        }
        PredictionMode::Vertical.predict::<SIDE>(&plane, stride, SIDE, 0, &mut output);
        assert_eq!(output, vec![127; SIDE * SIDE]);
        PredictionMode::Vertical.predict::<SIDE>(&plane, stride, SIDE, SIDE, &mut output);
        assert_eq!(output, vec![200; SIDE * SIDE]);
        PredictionMode::Horizontal.predict::<SIDE>(&plane, stride, 0, SIDE, &mut output);
        assert_eq!(output, vec![129; SIDE * SIDE]);
        PredictionMode::Horizontal.predict::<SIDE>(&plane, stride, SIDE, SIDE, &mut output);
        assert_eq!(output, vec![30; SIDE * SIDE]);
    }

    #[test]
    fn luma_modes_use_the_top_left_and_corner_values_at_all_frame_positions() {
        check_full_block_edges::<16>();
    }

    #[test]
    fn chroma_modes_use_the_top_left_and_corner_values_at_all_frame_positions() {
        check_full_block_edges::<8>();
    }

    #[test]
    fn true_motion_clamps_both_ends_of_the_pixel_range() {
        for (above, left, corner, expected) in [(250, 240, 10, 255), (10, 20, 240, 0)] {
            let edges = SubblockEdges {
                above: [above; 8],
                left: [left; 4],
                corner,
            };
            assert_eq!(edges.predict(SubblockMode::TrueMotion), [expected; 16]);
            for side in [8, 16] {
                let stride = side * 2;
                let mut plane = vec![0; stride * stride];
                plane[(side - 1) * stride + side..side * stride].fill(above);
                for row in side..stride {
                    plane[row * stride + side - 1] = left;
                }
                plane[(side - 1) * stride + side - 1] = corner;
                let mut output = vec![0; side * side];
                if side == 8 {
                    PredictionMode::TrueMotion.predict::<8>(
                        &plane,
                        stride,
                        side,
                        side,
                        &mut output,
                    );
                } else {
                    PredictionMode::TrueMotion.predict::<16>(
                        &plane,
                        stride,
                        side,
                        side,
                        &mut output,
                    );
                }
                assert_eq!(output, vec![expected; side * side]);
            }
        }
    }

    fn edge_plane() -> std::vec::Vec<u8> {
        (0..32 * 48)
            .map(|position| (position / 48 * 3 + position % 48) as u8)
            .collect()
    }

    #[test]
    fn right_edge_subblocks_reuse_the_macroblocks_above_right_pixels() {
        let plane = edge_plane();
        for y in [16, 20, 24, 28] {
            let edges = SubblockEdges::read(&plane, 48, 12, y);
            assert_eq!(&edges.above[4..], &[61, 62, 63, 64]);
        }
        assert_eq!(
            SubblockEdges::read(&plane, 48, 12, 20).above,
            [69, 70, 71, 72, 61, 62, 63, 64]
        );
    }

    #[test]
    fn rightmost_subblocks_extend_the_last_pixel_above_the_macroblock() {
        let plane = edge_plane();
        for y in [16, 20, 24, 28] {
            assert_eq!(&SubblockEdges::read(&plane, 48, 44, y).above[4..], &[92; 4]);
        }
    }

    #[test]
    fn top_row_macroblocks_supply_one_hundred_twenty_seven_above_right() {
        let plane = edge_plane();
        for x in [12, 28, 44] {
            for y in [0, 4, 8, 12] {
                assert_eq!(&SubblockEdges::read(&plane, 48, x, y).above[4..], &[127; 4]);
            }
        }
    }

    #[test]
    fn inner_subblocks_read_the_reconstructed_row_directly_above() {
        let plane = edge_plane();
        assert_eq!(
            SubblockEdges::read(&plane, 48, 8, 20).above,
            [65, 66, 67, 68, 69, 70, 71, 72]
        );
    }

    #[test]
    fn subblock_edges_apply_the_frame_border_and_corner_rules() {
        let plane = edge_plane();
        for (x, y, above, left, corner) in [
            (0, 0, [127; 8], [129; 4], 127),
            (4, 0, [127; 8], [3, 6, 9, 12], 127),
            (0, 4, [9, 10, 11, 12, 13, 14, 15, 16], [129; 4], 129),
            (4, 4, [13, 14, 15, 16, 17, 18, 19, 20], [15, 18, 21, 24], 12),
        ] {
            let edges = SubblockEdges::read(&plane, 48, x, y);
            assert_eq!(edges.above, above);
            assert_eq!(edges.left, left);
            assert_eq!(edges.corner, corner);
        }
    }

    #[test]
    fn registered_subblock_candidates_use_the_reconstruction_edges() {
        let plane = vec![0; 16 * 16];
        let mut output = [0; 16];
        for candidate in &CANDIDATES[4..] {
            candidate.mode.predict::<4>(&plane, 16, 0, 0, &mut output);
            let expected = match candidate.mode {
                PredictionMode::Subblock(SubblockMode::Dc) => [128; 16],
                PredictionMode::Subblock(SubblockMode::TrueMotion | SubblockMode::HorizontalUp) => {
                    [129; 16]
                }
                PredictionMode::Subblock(SubblockMode::HorizontalEdge) => [129; 16],
                PredictionMode::Subblock(SubblockMode::RightDown) => [
                    128, 127, 127, 127, 129, 128, 127, 127, 129, 129, 128, 127, 129, 129, 129, 128,
                ],
                PredictionMode::Subblock(SubblockMode::VerticalRight) => [
                    127, 127, 127, 127, 128, 127, 127, 127, 129, 127, 127, 127, 129, 128, 127, 127,
                ],
                PredictionMode::Subblock(SubblockMode::HorizontalDown) => [
                    128, 128, 127, 127, 129, 129, 128, 128, 129, 129, 129, 129, 129, 129, 129, 129,
                ],
                _ => [127; 16],
            };
            assert_eq!(output, expected, "{:?}", candidate.mode);
        }
    }
}
