use alloc::vec;
use alloc::vec::Vec;

use crate::bool_coder::BoolEncoder;

const PLANE_Y_AFTER_Y2: usize = 0;
const PLANE_Y2: usize = 1;
const PLANE_CHROMA: usize = 2;
const BAND_COUNT: usize = 8;
const CONTEXT_COUNT: usize = 3;
const TREE_NODE_COUNT: usize = 11;
const COEFF_PROB_COUNT: usize = 4 * BAND_COUNT * CONTEXT_COUNT * TREE_NODE_COUNT;

const TOKEN_ZERO: u8 = 0;
const TOKEN_ONE: u8 = 1;
const TOKEN_TWO: u8 = 2;
const TOKEN_THREE: u8 = 3;
const TOKEN_FOUR: u8 = 4;
const TOKEN_CATEGORY_ONE: u8 = 5;
const TOKEN_CATEGORY_TWO: u8 = 6;
const TOKEN_CATEGORY_THREE: u8 = 7;
const TOKEN_CATEGORY_FOUR: u8 = 8;
const TOKEN_CATEGORY_FIVE: u8 = 9;
const TOKEN_CATEGORY_SIX: u8 = 10;
const TOKEN_END: u8 = 11;

// RFC 6386 section 13.2 defines the coefficient token tree.
pub(crate) const COEFF_TREE: [i8; 22] = [
    -11, 2, 0, 4, -1, 6, 8, 12, -2, 10, -3, -4, 14, 16, -5, -6, 18, 20, -7, -8, -9, -10,
];

// RFC 6386 section 13.3 defines the coefficient bands.
const COEFF_BANDS: [usize; 16] = [0, 1, 2, 3, 6, 4, 5, 6, 6, 6, 6, 6, 6, 6, 6, 7];

// RFC 6386 section 13 names the zigzag scan and delegates its list to the attachment.
// The encoder writes raster positions in this order: 0, 1, 4, 8, 5, 2, 3, 6,
// 9, 12, 13, 10, 7, 11, 14, 15.
const ZIGZAG: [usize; 16] = [0, 1, 4, 8, 5, 2, 3, 6, 9, 12, 13, 10, 7, 11, 14, 15];

// RFC 6386 section 13.2 defines the category offset probabilities.
const CATEGORY_ONE_PROBS: [u8; 1] = [159];
const CATEGORY_TWO_PROBS: [u8; 2] = [165, 145];
const CATEGORY_THREE_PROBS: [u8; 3] = [173, 148, 140];
const CATEGORY_FOUR_PROBS: [u8; 4] = [176, 155, 140, 135];
const CATEGORY_FIVE_PROBS: [u8; 5] = [180, 157, 141, 134, 130];
const CATEGORY_SIX_PROBS: [u8; 11] = [254, 254, 243, 230, 196, 177, 153, 140, 133, 130, 129];

// RFC 6386 section 13.4 defines the coefficient update probabilities.
pub(crate) const COEFF_UPDATE_PROBS: [u8; COEFF_PROB_COUNT] = [
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 176, 246, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 223, 241, 252, 255, 255, 255, 255, 255, 255, 255, 255, 249, 253,
    253, 255, 255, 255, 255, 255, 255, 255, 255, 255, 244, 252, 255, 255, 255, 255, 255, 255, 255,
    255, 234, 254, 254, 255, 255, 255, 255, 255, 255, 255, 255, 253, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 246, 254, 255, 255, 255, 255, 255, 255, 255, 255, 239, 253, 254, 255,
    255, 255, 255, 255, 255, 255, 255, 254, 255, 254, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    248, 254, 255, 255, 255, 255, 255, 255, 255, 255, 251, 255, 254, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 253, 254, 255, 255, 255,
    255, 255, 255, 255, 255, 251, 254, 254, 255, 255, 255, 255, 255, 255, 255, 255, 254, 255, 254,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 254, 253, 255, 254, 255, 255, 255, 255, 255, 255,
    250, 255, 254, 255, 254, 255, 255, 255, 255, 255, 255, 254, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 217, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 225, 252, 241, 253, 255, 255, 254, 255, 255, 255,
    255, 234, 250, 241, 250, 253, 255, 253, 254, 255, 255, 255, 255, 254, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 223, 254, 254, 255, 255, 255, 255, 255, 255, 255, 255, 238, 253, 254, 254,
    255, 255, 255, 255, 255, 255, 255, 255, 248, 254, 255, 255, 255, 255, 255, 255, 255, 255, 249,
    254, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 253, 255, 255, 255, 255, 255, 255, 255, 255, 255, 247, 254, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 253, 254,
    255, 255, 255, 255, 255, 255, 255, 255, 252, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 254, 254, 255, 255, 255, 255, 255,
    255, 255, 255, 253, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 254, 253, 255, 255, 255, 255, 255, 255, 255, 255, 250, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 254, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 186, 251, 250, 255,
    255, 255, 255, 255, 255, 255, 255, 234, 251, 244, 254, 255, 255, 255, 255, 255, 255, 255, 251,
    251, 243, 253, 254, 255, 254, 255, 255, 255, 255, 255, 253, 254, 255, 255, 255, 255, 255, 255,
    255, 255, 236, 253, 254, 255, 255, 255, 255, 255, 255, 255, 255, 251, 253, 253, 254, 254, 255,
    255, 255, 255, 255, 255, 255, 254, 254, 255, 255, 255, 255, 255, 255, 255, 255, 254, 254, 254,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 254, 255, 255, 255, 255, 255, 255, 255, 255, 255, 254, 254, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 254, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 254, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 248, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 250, 254, 252, 254, 255, 255, 255, 255, 255, 255, 255, 248, 254, 249,
    253, 255, 255, 255, 255, 255, 255, 255, 255, 253, 253, 255, 255, 255, 255, 255, 255, 255, 255,
    246, 253, 253, 255, 255, 255, 255, 255, 255, 255, 255, 252, 254, 251, 254, 254, 255, 255, 255,
    255, 255, 255, 255, 254, 252, 255, 255, 255, 255, 255, 255, 255, 255, 248, 254, 253, 255, 255,
    255, 255, 255, 255, 255, 255, 253, 255, 254, 254, 255, 255, 255, 255, 255, 255, 255, 255, 251,
    254, 255, 255, 255, 255, 255, 255, 255, 255, 245, 251, 254, 255, 255, 255, 255, 255, 255, 255,
    255, 253, 253, 254, 255, 255, 255, 255, 255, 255, 255, 255, 255, 251, 253, 255, 255, 255, 255,
    255, 255, 255, 255, 252, 253, 254, 255, 255, 255, 255, 255, 255, 255, 255, 255, 254, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 252, 255, 255, 255, 255, 255, 255, 255, 255, 255, 249,
    255, 254, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 254, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 253, 255, 255, 255, 255, 255, 255, 255, 255, 250, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 254, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
];

// RFC 6386 section 13.5 defines the default coefficient probabilities.
pub(crate) const DEFAULT_COEFF_PROBS: [u8; COEFF_PROB_COUNT] = [
    128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128,
    128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 253, 136, 254, 255, 228,
    219, 128, 128, 128, 128, 128, 189, 129, 242, 255, 227, 213, 255, 219, 128, 128, 128, 106, 126,
    227, 252, 214, 209, 255, 255, 128, 128, 128, 1, 98, 248, 255, 236, 226, 255, 255, 128, 128,
    128, 181, 133, 238, 254, 221, 234, 255, 154, 128, 128, 128, 78, 134, 202, 247, 198, 180, 255,
    219, 128, 128, 128, 1, 185, 249, 255, 243, 255, 128, 128, 128, 128, 128, 184, 150, 247, 255,
    236, 224, 128, 128, 128, 128, 128, 77, 110, 216, 255, 236, 230, 128, 128, 128, 128, 128, 1,
    101, 251, 255, 241, 255, 128, 128, 128, 128, 128, 170, 139, 241, 252, 236, 209, 255, 255, 128,
    128, 128, 37, 116, 196, 243, 228, 255, 255, 255, 128, 128, 128, 1, 204, 254, 255, 245, 255,
    128, 128, 128, 128, 128, 207, 160, 250, 255, 238, 128, 128, 128, 128, 128, 128, 102, 103, 231,
    255, 211, 171, 128, 128, 128, 128, 128, 1, 152, 252, 255, 240, 255, 128, 128, 128, 128, 128,
    177, 135, 243, 255, 234, 225, 128, 128, 128, 128, 128, 80, 129, 211, 255, 194, 224, 128, 128,
    128, 128, 128, 1, 1, 255, 128, 128, 128, 128, 128, 128, 128, 128, 246, 1, 255, 128, 128, 128,
    128, 128, 128, 128, 128, 255, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 198, 35, 237,
    223, 193, 187, 162, 160, 145, 155, 62, 131, 45, 198, 221, 172, 176, 220, 157, 252, 221, 1, 68,
    47, 146, 208, 149, 167, 221, 162, 255, 223, 128, 1, 149, 241, 255, 221, 224, 255, 255, 128,
    128, 128, 184, 141, 234, 253, 222, 220, 255, 199, 128, 128, 128, 81, 99, 181, 242, 176, 190,
    249, 202, 255, 255, 128, 1, 129, 232, 253, 214, 197, 242, 196, 255, 255, 128, 99, 121, 210,
    250, 201, 198, 255, 202, 128, 128, 128, 23, 91, 163, 242, 170, 187, 247, 210, 255, 255, 128, 1,
    200, 246, 255, 234, 255, 128, 128, 128, 128, 128, 109, 178, 241, 255, 231, 245, 255, 255, 128,
    128, 128, 44, 130, 201, 253, 205, 192, 255, 255, 128, 128, 128, 1, 132, 239, 251, 219, 209,
    255, 165, 128, 128, 128, 94, 136, 225, 251, 218, 190, 255, 255, 128, 128, 128, 22, 100, 174,
    245, 186, 161, 255, 199, 128, 128, 128, 1, 182, 249, 255, 232, 235, 128, 128, 128, 128, 128,
    124, 143, 241, 255, 227, 234, 128, 128, 128, 128, 128, 35, 77, 181, 251, 193, 211, 255, 205,
    128, 128, 128, 1, 157, 247, 255, 236, 231, 255, 255, 128, 128, 128, 121, 141, 235, 255, 225,
    227, 255, 255, 128, 128, 128, 45, 99, 188, 251, 195, 217, 255, 224, 128, 128, 128, 1, 1, 251,
    255, 213, 255, 128, 128, 128, 128, 128, 203, 1, 248, 255, 255, 128, 128, 128, 128, 128, 128,
    137, 1, 177, 255, 224, 255, 128, 128, 128, 128, 128, 253, 9, 248, 251, 207, 208, 255, 192, 128,
    128, 128, 175, 13, 224, 243, 193, 185, 249, 198, 255, 255, 128, 73, 17, 171, 221, 161, 179,
    236, 167, 255, 234, 128, 1, 95, 247, 253, 212, 183, 255, 255, 128, 128, 128, 239, 90, 244, 250,
    211, 209, 255, 255, 128, 128, 128, 155, 77, 195, 248, 188, 195, 255, 255, 128, 128, 128, 1, 24,
    239, 251, 218, 219, 255, 205, 128, 128, 128, 201, 51, 219, 255, 196, 186, 128, 128, 128, 128,
    128, 69, 46, 190, 239, 201, 218, 255, 228, 128, 128, 128, 1, 191, 251, 255, 255, 128, 128, 128,
    128, 128, 128, 223, 165, 249, 255, 213, 255, 128, 128, 128, 128, 128, 141, 124, 248, 255, 255,
    128, 128, 128, 128, 128, 128, 1, 16, 248, 255, 255, 128, 128, 128, 128, 128, 128, 190, 36, 230,
    255, 236, 255, 128, 128, 128, 128, 128, 149, 1, 255, 128, 128, 128, 128, 128, 128, 128, 128, 1,
    226, 255, 128, 128, 128, 128, 128, 128, 128, 128, 247, 192, 255, 128, 128, 128, 128, 128, 128,
    128, 128, 240, 128, 255, 128, 128, 128, 128, 128, 128, 128, 128, 1, 134, 252, 255, 255, 128,
    128, 128, 128, 128, 128, 213, 62, 250, 255, 255, 128, 128, 128, 128, 128, 128, 55, 93, 255,
    128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128,
    128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128,
    128, 128, 128, 202, 24, 213, 235, 186, 191, 220, 160, 240, 175, 255, 126, 38, 182, 232, 169,
    184, 228, 174, 255, 187, 128, 61, 46, 138, 219, 151, 178, 240, 170, 255, 216, 128, 1, 112, 230,
    250, 199, 191, 247, 159, 255, 255, 128, 166, 109, 228, 252, 211, 215, 255, 174, 128, 128, 128,
    39, 77, 162, 232, 172, 180, 245, 178, 255, 255, 128, 1, 52, 220, 246, 198, 199, 249, 220, 255,
    255, 128, 124, 74, 191, 243, 183, 193, 250, 221, 255, 255, 128, 24, 71, 130, 219, 154, 170,
    243, 182, 255, 255, 128, 1, 182, 225, 249, 219, 240, 255, 224, 128, 128, 128, 149, 150, 226,
    252, 216, 205, 255, 171, 128, 128, 128, 28, 108, 170, 242, 183, 194, 254, 223, 255, 255, 128,
    1, 81, 230, 252, 204, 203, 255, 192, 128, 128, 128, 123, 102, 209, 247, 188, 196, 255, 233,
    128, 128, 128, 20, 95, 153, 243, 164, 173, 255, 203, 128, 128, 128, 1, 222, 248, 255, 216, 213,
    128, 128, 128, 128, 128, 168, 175, 246, 252, 235, 205, 255, 255, 128, 128, 128, 47, 116, 215,
    255, 211, 212, 255, 255, 128, 128, 128, 1, 121, 236, 253, 212, 214, 255, 255, 128, 128, 128,
    141, 84, 213, 252, 201, 202, 255, 219, 128, 128, 128, 42, 80, 160, 240, 162, 185, 255, 205,
    128, 128, 128, 1, 1, 255, 128, 128, 128, 128, 128, 128, 128, 128, 244, 1, 255, 128, 128, 128,
    128, 128, 128, 128, 128, 238, 1, 255, 128, 128, 128, 128, 128, 128, 128, 128,
];

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct MacroblockResidual {
    pub(crate) y2: [i16; 16],
    pub(crate) y: [[i16; 16]; 16],
    pub(crate) u: [[i16; 16]; 4],
    pub(crate) v: [[i16; 16]; 4],
}

pub(crate) struct ResidualWriter {
    above_y: Vec<bool>,
    above_u: Vec<bool>,
    above_v: Vec<bool>,
    above_y2: Vec<bool>,
    left_y: [bool; 4],
    left_u: [bool; 2],
    left_v: [bool; 2],
    left_y2: bool,
}

impl ResidualWriter {
    pub(crate) fn new(macroblock_columns: usize) -> Self {
        Self {
            above_y: vec![false; macroblock_columns * 4],
            above_u: vec![false; macroblock_columns * 2],
            above_v: vec![false; macroblock_columns * 2],
            above_y2: vec![false; macroblock_columns],
            left_y: [false; 4],
            left_u: [false; 2],
            left_v: [false; 2],
            left_y2: false,
        }
    }

    pub(crate) fn write_macroblock(
        &mut self,
        encoder: &mut BoolEncoder,
        macroblock_x: usize,
        residual: &MacroblockResidual,
    ) {
        if macroblock_x == 0 {
            self.left_y = [false; 4];
            self.left_u = [false; 2];
            self.left_v = [false; 2];
            self.left_y2 = false;
        }

        let y2_context = usize::from(self.left_y2) + usize::from(self.above_y2[macroblock_x]);
        let y2_has_coefficients = write_block(encoder, &residual.y2, PLANE_Y2, 0, y2_context);
        self.left_y2 = y2_has_coefficients;
        self.above_y2[macroblock_x] = y2_has_coefficients;

        write_plane_blocks(
            encoder,
            &residual.y,
            PlaneContext {
                blocks_wide: 4,
                above: &mut self.above_y,
                above_start: macroblock_x * 4,
                left: &mut self.left_y,
                plane: PLANE_Y_AFTER_Y2,
                first_position: 1,
            },
        );
        write_plane_blocks(
            encoder,
            &residual.u,
            PlaneContext {
                blocks_wide: 2,
                above: &mut self.above_u,
                above_start: macroblock_x * 2,
                left: &mut self.left_u,
                plane: PLANE_CHROMA,
                first_position: 0,
            },
        );
        write_plane_blocks(
            encoder,
            &residual.v,
            PlaneContext {
                blocks_wide: 2,
                above: &mut self.above_v,
                above_start: macroblock_x * 2,
                left: &mut self.left_v,
                plane: PLANE_CHROMA,
                first_position: 0,
            },
        );
    }
}

struct PlaneContext<'a> {
    blocks_wide: usize,
    above: &'a mut [bool],
    above_start: usize,
    left: &'a mut [bool],
    plane: usize,
    first_position: usize,
}

fn write_plane_blocks<W: EntropyWriter>(
    writer: &mut W,
    blocks: &[[i16; 16]],
    context: PlaneContext<'_>,
) {
    for (block_index, block) in blocks.iter().enumerate() {
        let block_x = block_index % context.blocks_wide;
        let block_y = block_index / context.blocks_wide;
        let neighbor_count = usize::from(context.left[block_y])
            + usize::from(context.above[context.above_start + block_x]);
        let has_coefficients = write_block(
            writer,
            block,
            context.plane,
            context.first_position,
            neighbor_count,
        );
        context.left[block_y] = has_coefficients;
        context.above[context.above_start + block_x] = has_coefficients;
    }
}

fn write_block<W: EntropyWriter>(
    writer: &mut W,
    levels: &[i16; 16],
    plane: usize,
    first_position: usize,
    first_context: usize,
) -> bool {
    let last_nonzero = (first_position..16)
        .rev()
        .find(|position| levels[ZIGZAG[*position]] != 0);
    let Some(last_nonzero) = last_nonzero else {
        write_token(
            writer,
            TOKEN_END,
            plane,
            first_position,
            first_context,
            false,
        );
        return false;
    };

    let mut context = first_context;
    let mut previous_was_zero = false;
    for position in first_position..=last_nonzero {
        let level = levels[ZIGZAG[position]];
        let magnitude = level.unsigned_abs().min(2114);
        let token = token_for_magnitude(magnitude);
        write_token(writer, token, plane, position, context, previous_was_zero);
        if magnitude != 0 {
            write_extra_bits(writer, magnitude, token);
            writer.write_bool(128, level < 0);
        }
        context = magnitude_context(magnitude);
        previous_was_zero = magnitude == 0;
    }

    if last_nonzero < 15 {
        write_token(writer, TOKEN_END, plane, last_nonzero + 1, context, false);
    }
    true
}

fn write_token<W: EntropyWriter>(
    writer: &mut W,
    token: u8,
    plane: usize,
    position: usize,
    context: usize,
    skip_end_branch: bool,
) {
    let probabilities = coefficient_probabilities(plane, COEFF_BANDS[position], context);
    let start_node = if skip_end_branch { 2 } else { 0 };
    writer.write_tree(&COEFF_TREE, probabilities, token, start_node);
}

fn write_extra_bits<W: EntropyWriter>(writer: &mut W, magnitude: u16, token: u8) {
    let (base, probabilities): (u16, &[u8]) = match token {
        TOKEN_CATEGORY_ONE => (5, &CATEGORY_ONE_PROBS),
        TOKEN_CATEGORY_TWO => (7, &CATEGORY_TWO_PROBS),
        TOKEN_CATEGORY_THREE => (11, &CATEGORY_THREE_PROBS),
        TOKEN_CATEGORY_FOUR => (19, &CATEGORY_FOUR_PROBS),
        TOKEN_CATEGORY_FIVE => (35, &CATEGORY_FIVE_PROBS),
        TOKEN_CATEGORY_SIX => (67, &CATEGORY_SIX_PROBS),
        _ => return,
    };
    let extra = magnitude - base;
    for (index, probability) in probabilities.iter().enumerate() {
        let shift = probabilities.len() - index - 1;
        writer.write_bool(*probability, extra & (1 << shift) != 0);
    }
}

fn token_for_magnitude(magnitude: u16) -> u8 {
    match magnitude {
        0 => TOKEN_ZERO,
        1 => TOKEN_ONE,
        2 => TOKEN_TWO,
        3 => TOKEN_THREE,
        4 => TOKEN_FOUR,
        5..=6 => TOKEN_CATEGORY_ONE,
        7..=10 => TOKEN_CATEGORY_TWO,
        11..=18 => TOKEN_CATEGORY_THREE,
        19..=34 => TOKEN_CATEGORY_FOUR,
        35..=66 => TOKEN_CATEGORY_FIVE,
        67..=2114 => TOKEN_CATEGORY_SIX,
        _ => unreachable!("quantized levels stop at 2047"),
    }
}

fn magnitude_context(magnitude: u16) -> usize {
    match magnitude {
        0 => 0,
        1 => 1,
        _ => 2,
    }
}

fn coefficient_probabilities(plane: usize, band: usize, context: usize) -> &'static [u8] {
    let start = ((plane * BAND_COUNT + band) * CONTEXT_COUNT + context) * TREE_NODE_COUNT;
    &DEFAULT_COEFF_PROBS[start..start + TREE_NODE_COUNT]
}

trait EntropyWriter {
    fn write_bool(&mut self, probability: u8, value: bool);

    fn write_tree(&mut self, tree: &[i8], probabilities: &[u8], value: u8, start_node: usize);
}

impl EntropyWriter for BoolEncoder {
    fn write_bool(&mut self, probability: u8, value: bool) {
        BoolEncoder::write_bool(self, probability, value);
    }

    fn write_tree(&mut self, tree: &[i8], probabilities: &[u8], value: u8, start_node: usize) {
        BoolEncoder::write_tree(self, tree, probabilities, value, start_node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bool_coder::BoolDecoder;

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct ContextState {
        above_y: Vec<bool>,
        above_u: Vec<bool>,
        above_v: Vec<bool>,
        above_y2: Vec<bool>,
        left_y: [bool; 4],
        left_u: [bool; 2],
        left_v: [bool; 2],
        left_y2: bool,
    }

    impl From<&ResidualWriter> for ContextState {
        fn from(writer: &ResidualWriter) -> Self {
            Self {
                above_y: writer.above_y.clone(),
                above_u: writer.above_u.clone(),
                above_v: writer.above_v.clone(),
                above_y2: writer.above_y2.clone(),
                left_y: writer.left_y,
                left_u: writer.left_u,
                left_v: writer.left_v,
                left_y2: writer.left_y2,
            }
        }
    }

    struct ResidualReader {
        above_y: Vec<bool>,
        above_u: Vec<bool>,
        above_v: Vec<bool>,
        above_y2: Vec<bool>,
        left_y: [bool; 4],
        left_u: [bool; 2],
        left_v: [bool; 2],
        left_y2: bool,
        first_contexts_seen: [bool; 3],
    }

    impl ResidualReader {
        fn new(macroblock_columns: usize) -> Self {
            Self {
                above_y: vec![false; macroblock_columns * 4],
                above_u: vec![false; macroblock_columns * 2],
                above_v: vec![false; macroblock_columns * 2],
                above_y2: vec![false; macroblock_columns],
                left_y: [false; 4],
                left_u: [false; 2],
                left_v: [false; 2],
                left_y2: false,
                first_contexts_seen: [false; 3],
            }
        }

        fn read_macroblock(
            &mut self,
            decoder: &mut BoolDecoder<'_>,
            macroblock_x: usize,
        ) -> MacroblockResidual {
            if macroblock_x == 0 {
                self.left_y = [false; 4];
                self.left_u = [false; 2];
                self.left_v = [false; 2];
                self.left_y2 = false;
            }

            let mut residual = MacroblockResidual::default();
            let y2_context = usize::from(self.left_y2) + usize::from(self.above_y2[macroblock_x]);
            self.first_contexts_seen[y2_context] = true;
            let (y2, y2_has_coefficients) = read_block(decoder, PLANE_Y2, 0, y2_context);
            residual.y2 = y2;
            self.left_y2 = y2_has_coefficients;
            self.above_y2[macroblock_x] = y2_has_coefficients;

            read_plane_blocks(
                decoder,
                &mut residual.y,
                &mut self.first_contexts_seen,
                PlaneContext {
                    blocks_wide: 4,
                    above: &mut self.above_y,
                    above_start: macroblock_x * 4,
                    left: &mut self.left_y,
                    plane: PLANE_Y_AFTER_Y2,
                    first_position: 1,
                },
            );
            read_plane_blocks(
                decoder,
                &mut residual.u,
                &mut self.first_contexts_seen,
                PlaneContext {
                    blocks_wide: 2,
                    above: &mut self.above_u,
                    above_start: macroblock_x * 2,
                    left: &mut self.left_u,
                    plane: PLANE_CHROMA,
                    first_position: 0,
                },
            );
            read_plane_blocks(
                decoder,
                &mut residual.v,
                &mut self.first_contexts_seen,
                PlaneContext {
                    blocks_wide: 2,
                    above: &mut self.above_v,
                    above_start: macroblock_x * 2,
                    left: &mut self.left_v,
                    plane: PLANE_CHROMA,
                    first_position: 0,
                },
            );
            residual
        }

        fn state(&self) -> ContextState {
            ContextState {
                above_y: self.above_y.clone(),
                above_u: self.above_u.clone(),
                above_v: self.above_v.clone(),
                above_y2: self.above_y2.clone(),
                left_y: self.left_y,
                left_u: self.left_u,
                left_v: self.left_v,
                left_y2: self.left_y2,
            }
        }
    }

    fn read_plane_blocks(
        decoder: &mut BoolDecoder<'_>,
        blocks: &mut [[i16; 16]],
        first_contexts_seen: &mut [bool; 3],
        context: PlaneContext<'_>,
    ) {
        for (block_index, block) in blocks.iter_mut().enumerate() {
            let block_x = block_index % context.blocks_wide;
            let block_y = block_index / context.blocks_wide;
            let neighbor_count = usize::from(context.left[block_y])
                + usize::from(context.above[context.above_start + block_x]);
            first_contexts_seen[neighbor_count] = true;
            let (decoded, has_coefficients) = read_block(
                decoder,
                context.plane,
                context.first_position,
                neighbor_count,
            );
            *block = decoded;
            context.left[block_y] = has_coefficients;
            context.above[context.above_start + block_x] = has_coefficients;
        }
    }

    fn read_block(
        decoder: &mut BoolDecoder<'_>,
        plane: usize,
        first_position: usize,
        first_context: usize,
    ) -> ([i16; 16], bool) {
        let mut levels = [0; 16];
        let mut context = first_context;
        let mut previous_was_zero = false;
        let mut has_coefficients = false;

        for position in first_position..16 {
            let token = read_token(decoder, plane, position, context, previous_was_zero);
            if token == TOKEN_END {
                break;
            }

            let magnitude = read_magnitude(decoder, token);
            if magnitude != 0 {
                has_coefficients = true;
                let negative = decoder.read_bool(128);
                levels[ZIGZAG[position]] = if negative {
                    -(magnitude as i16)
                } else {
                    magnitude as i16
                };
            }
            context = magnitude_context(magnitude);
            previous_was_zero = magnitude == 0;
        }
        (levels, has_coefficients)
    }

    fn read_token(
        decoder: &mut BoolDecoder<'_>,
        plane: usize,
        position: usize,
        context: usize,
        skip_end_branch: bool,
    ) -> u8 {
        let probabilities = coefficient_probabilities(plane, COEFF_BANDS[position], context);
        let start_node = if skip_end_branch { 2 } else { 0 };
        decoder.read_tree(&COEFF_TREE, probabilities, start_node)
    }

    fn read_magnitude(decoder: &mut BoolDecoder<'_>, token: u8) -> u16 {
        let (base, probabilities): (u16, &[u8]) = match token {
            TOKEN_ZERO => return 0,
            TOKEN_ONE => return 1,
            TOKEN_TWO => return 2,
            TOKEN_THREE => return 3,
            TOKEN_FOUR => return 4,
            TOKEN_CATEGORY_ONE => (5, &CATEGORY_ONE_PROBS),
            TOKEN_CATEGORY_TWO => (7, &CATEGORY_TWO_PROBS),
            TOKEN_CATEGORY_THREE => (11, &CATEGORY_THREE_PROBS),
            TOKEN_CATEGORY_FOUR => (19, &CATEGORY_FOUR_PROBS),
            TOKEN_CATEGORY_FIVE => (35, &CATEGORY_FIVE_PROBS),
            TOKEN_CATEGORY_SIX => (67, &CATEGORY_SIX_PROBS),
            _ => unreachable!("the end token leaves the block before magnitude decoding"),
        };
        let mut extra = 0u16;
        for probability in probabilities {
            extra = (extra << 1) | u16::from(decoder.read_bool(*probability));
        }
        base + extra
    }

    #[derive(Default)]
    struct TraceWriter {
        writes: Vec<(u8, bool)>,
    }

    impl EntropyWriter for TraceWriter {
        fn write_bool(&mut self, probability: u8, value: bool) {
            self.writes.push((probability, value));
        }

        fn write_tree(&mut self, tree: &[i8], probabilities: &[u8], value: u8, start_node: usize) {
            let (writes, write_count) =
                crate::bool_coder::tree_writes(tree, probabilities, value, start_node);
            self.writes.extend_from_slice(&writes[..write_count]);
        }
    }

    fn next_seed(seed: &mut u32) -> u32 {
        *seed ^= *seed << 13;
        *seed ^= *seed >> 17;
        *seed ^= *seed << 5;
        *seed
    }

    fn fill_block(block: &mut [i16; 16], first_position: usize, sparsity: usize, seed: &mut u32) {
        let mut order = ZIGZAG;
        for position in (first_position + 1..16).rev() {
            let swap_with =
                first_position + next_seed(seed) as usize % (position + 1 - first_position);
            order.swap(position, swap_with);
        }
        let count = sparsity.min(16 - first_position);
        for (selected, raster_position) in order[first_position..first_position + count]
            .iter()
            .enumerate()
        {
            let magnitude = if selected == 0 && next_seed(seed) & 7 == 0 {
                2047
            } else {
                (next_seed(seed) % 2047 + 1) as i16
            };
            block[*raster_position] = if next_seed(seed) & 1 == 0 {
                magnitude
            } else {
                -magnitude
            };
        }
    }

    fn seeded_macroblock(seed: &mut u32) -> MacroblockResidual {
        let mut residual = MacroblockResidual::default();
        let sparsity = next_seed(seed) as usize % 17;
        fill_block(&mut residual.y2, 0, sparsity, seed);
        for block in &mut residual.y {
            let sparsity = next_seed(seed) as usize % 17;
            fill_block(block, 1, sparsity, seed);
        }
        for block in &mut residual.u {
            let sparsity = next_seed(seed) as usize % 17;
            fill_block(block, 0, sparsity, seed);
        }
        for block in &mut residual.v {
            let sparsity = next_seed(seed) as usize % 17;
            fill_block(block, 0, sparsity, seed);
        }
        residual
    }

    #[test]
    fn the_update_probability_table_matches_the_section_13_4_pins() {
        assert_eq!(COEFF_UPDATE_PROBS.len(), 4 * 8 * 3 * 11);
        assert_eq!(COEFF_UPDATE_PROBS.first(), Some(&255));
        assert_eq!(COEFF_UPDATE_PROBS.last(), Some(&255));
        assert_eq!(
            COEFF_UPDATE_PROBS
                .iter()
                .map(|value| u32::from(*value))
                .sum::<u32>(),
            268_469
        );
    }

    #[test]
    fn the_default_probability_table_matches_the_section_13_5_pins() {
        assert_eq!(DEFAULT_COEFF_PROBS.len(), 4 * 8 * 3 * 11);
        assert_eq!(DEFAULT_COEFF_PROBS.first(), Some(&128));
        assert_eq!(DEFAULT_COEFF_PROBS.last(), Some(&128));
        assert_eq!(
            DEFAULT_COEFF_PROBS
                .iter()
                .map(|value| u32::from(*value))
                .sum::<u32>(),
            174_918
        );
    }

    #[test]
    fn the_band_and_zigzag_tables_match_the_section_13_pins() {
        assert_eq!(COEFF_BANDS.len(), 16);
        assert_eq!(COEFF_BANDS.first(), Some(&0));
        assert_eq!(COEFF_BANDS.last(), Some(&7));
        assert_eq!(COEFF_BANDS.iter().sum::<usize>(), 76);
        assert_eq!(ZIGZAG.len(), 16);
        assert_eq!(ZIGZAG.first(), Some(&0));
        assert_eq!(ZIGZAG.last(), Some(&15));
        assert_eq!(ZIGZAG.iter().sum::<usize>(), 120);
    }

    #[test]
    fn the_category_probability_tables_match_the_section_13_2_pins() {
        assert_eq!(CATEGORY_ONE_PROBS, [159]);
        assert_eq!(CATEGORY_TWO_PROBS, [165, 145]);
        assert_eq!(CATEGORY_THREE_PROBS, [173, 148, 140]);
        assert_eq!(CATEGORY_FOUR_PROBS, [176, 155, 140, 135]);
        assert_eq!(CATEGORY_FIVE_PROBS, [180, 157, 141, 134, 130]);
        assert_eq!(
            CATEGORY_SIX_PROBS,
            [254, 254, 243, 230, 196, 177, 153, 140, 133, 130, 129]
        );
        assert_eq!(
            [
                CATEGORY_ONE_PROBS
                    .iter()
                    .map(|value| u32::from(*value))
                    .sum::<u32>(),
                CATEGORY_TWO_PROBS
                    .iter()
                    .map(|value| u32::from(*value))
                    .sum::<u32>(),
                CATEGORY_THREE_PROBS
                    .iter()
                    .map(|value| u32::from(*value))
                    .sum::<u32>(),
                CATEGORY_FOUR_PROBS
                    .iter()
                    .map(|value| u32::from(*value))
                    .sum::<u32>(),
                CATEGORY_FIVE_PROBS
                    .iter()
                    .map(|value| u32::from(*value))
                    .sum::<u32>(),
                CATEGORY_SIX_PROBS
                    .iter()
                    .map(|value| u32::from(*value))
                    .sum::<u32>(),
            ],
            [159, 310, 461, 606, 742, 2039]
        );
    }

    #[test]
    fn every_coefficient_token_follows_the_section_13_2_tree_path() {
        let expected = vec![
            vec![true, false],
            vec![true, true, false],
            vec![true, true, true, false, false],
            vec![true, true, true, false, true, false],
            vec![true, true, true, false, true, true],
            vec![true, true, true, true, false, false],
            vec![true, true, true, true, false, true],
            vec![true, true, true, true, true, false, false],
            vec![true, true, true, true, true, false, true],
            vec![true, true, true, true, true, true, false],
            vec![true, true, true, true, true, true, true],
            vec![false],
        ];
        let actual: Vec<Vec<bool>> = (TOKEN_ZERO..=TOKEN_END)
            .map(|token| {
                let (writes, write_count) = crate::bool_coder::tree_writes(
                    &COEFF_TREE,
                    &DEFAULT_COEFF_PROBS[..TREE_NODE_COUNT],
                    token,
                    0,
                );
                writes[..write_count]
                    .iter()
                    .map(|(_, branch)| *branch)
                    .collect()
            })
            .collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn an_all_zero_y_block_at_context_zero_writes_one_bool_at_probability_253() {
        let mut writer = TraceWriter::default();
        let has_coefficients = write_block(&mut writer, &[0; 16], PLANE_Y_AFTER_Y2, 1, 0);
        assert_eq!(
            (has_coefficients, writer.writes),
            (false, vec![(253, false)])
        );
    }

    #[test]
    fn magnitudes_above_2114_saturate_at_2114() {
        let mut levels = [0; 16];
        levels[0] = i16::MAX;
        levels[1] = i16::MIN;
        let mut encoder = BoolEncoder::new();
        assert_eq!(
            u8::from(write_block(&mut encoder, &levels, PLANE_CHROMA, 0, 0)),
            1
        );
        let encoded = encoder.finish();
        let mut decoder = BoolDecoder::new(&encoded);
        let (decoded, has_coefficients) = read_block(&mut decoder, PLANE_CHROMA, 0, 0);
        let mut expected = [0; 16];
        expected[0] = 2114;
        expected[1] = -2114;
        assert_eq!((decoded, has_coefficients), (expected, true));
    }

    #[test]
    fn every_seeded_macroblock_sequence_reads_back_with_all_first_contexts() {
        const COLUMNS: usize = 5;
        const MACROBLOCKS: usize = COLUMNS * 20;

        let mut seed = 0x6d2b_79f5u32;
        let expected: Vec<_> = (0..MACROBLOCKS)
            .map(|_| seeded_macroblock(&mut seed))
            .collect();
        let mut encoder = BoolEncoder::new();
        let mut writer = ResidualWriter::new(COLUMNS);
        let mut states = Vec::with_capacity(MACROBLOCKS);
        for (index, residual) in expected.iter().enumerate() {
            writer.write_macroblock(&mut encoder, index % COLUMNS, residual);
            states.push(ContextState::from(&writer));
        }

        let encoded = encoder.finish();
        let mut decoder = BoolDecoder::new(&encoded);
        let mut reader = ResidualReader::new(COLUMNS);
        for (index, residual) in expected.iter().enumerate() {
            let decoded = reader.read_macroblock(&mut decoder, index % COLUMNS);
            assert_eq!(decoded, *residual, "macroblock {index}");
            assert_eq!(
                reader.state(),
                states[index],
                "context after macroblock {index}"
            );
        }
        assert_eq!(reader.first_contexts_seen, [true; 3]);
    }
}
