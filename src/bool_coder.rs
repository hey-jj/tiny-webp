use alloc::vec::Vec;

const MAX_TREE_DEPTH: usize = 11;

pub(crate) struct BoolEncoder {
    output: Vec<u8>,
    range: u32,
    bottom: u32,
    bit_count: u8,
}

impl BoolEncoder {
    #[cfg(test)]
    pub(crate) fn new() -> Self {
        Self::with_capacity(0)
    }

    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            output: Vec::with_capacity(capacity),
            range: 255,
            bottom: 0,
            bit_count: 24,
        }
    }

    pub(crate) fn write_bool(&mut self, probability: u8, value: bool) {
        debug_assert_ne!(probability, 0);
        let split = 1 + (((self.range - 1) * u32::from(probability)) >> 8);

        if value {
            self.bottom += split;
            self.range -= split;
        } else {
            self.range = split;
        }

        while self.range < 128 {
            self.range <<= 1;
            if self.bottom & (1 << 31) != 0 {
                self.propagate_carry();
            }
            self.bottom <<= 1;
            self.bit_count -= 1;
            if self.bit_count == 0 {
                self.output.push((self.bottom >> 24) as u8);
                self.bottom &= (1 << 24) - 1;
                self.bit_count = 8;
            }
        }
    }

    pub(crate) fn write_literal(&mut self, value: u32, bit_count: u8) {
        for shift in (0..bit_count).rev() {
            self.write_bool(128, value & (1 << shift) != 0);
        }
    }

    pub(crate) fn write_tree(
        &mut self,
        tree: &[i8],
        probabilities: &[u8],
        value: u8,
        start_node: usize,
    ) {
        let (writes, write_count) = tree_writes(tree, probabilities, value, start_node);
        for (probability, branch) in writes.into_iter().take(write_count) {
            self.write_bool(probability, branch);
        }
    }

    pub(crate) fn finish(mut self) -> Vec<u8> {
        let mut remaining = self.bit_count;
        let mut value = self.bottom;

        if value & (1 << (32 - remaining)) != 0 {
            self.propagate_carry();
        }
        value <<= remaining & 7;
        remaining >>= 3;
        while remaining > 0 {
            value <<= 8;
            remaining -= 1;
        }
        for _ in 0..4 {
            self.output.push((value >> 24) as u8);
            value <<= 8;
        }
        self.output
    }

    fn propagate_carry(&mut self) {
        for byte in self.output.iter_mut().rev() {
            if *byte == 255 {
                *byte = 0;
            } else {
                *byte += 1;
                return;
            }
        }
        unreachable!("the coded value stays below one");
    }
}

pub(crate) fn tree_writes(
    tree: &[i8],
    probabilities: &[u8],
    value: u8,
    start_node: usize,
) -> ([(u8, bool); MAX_TREE_DEPTH], usize) {
    let mut path = [false; MAX_TREE_DEPTH];
    let path_len = find_tree_path(tree, start_node, value, &mut path, 0)
        .expect("the value must be a leaf in the tree");
    let mut writes = [(0, false); MAX_TREE_DEPTH];
    let mut node = start_node;

    for (index, branch) in path.into_iter().take(path_len).enumerate() {
        writes[index] = (probabilities[node >> 1], branch);
        node = tree[node + usize::from(branch)] as usize;
    }
    (writes, path_len)
}

fn find_tree_path(
    tree: &[i8],
    node: usize,
    value: u8,
    path: &mut [bool; MAX_TREE_DEPTH],
    depth: usize,
) -> Option<usize> {
    for branch in [false, true] {
        let child = tree[node + usize::from(branch)];
        path[depth] = branch;
        if child <= 0 {
            if child.unsigned_abs() == value {
                return Some(depth + 1);
            }
        } else if let Some(path_len) = find_tree_path(tree, child as usize, value, path, depth + 1)
        {
            return Some(path_len);
        }
    }
    None
}

#[cfg(test)]
pub(crate) struct BoolDecoder<'a> {
    input: &'a [u8],
    position: usize,
    range: u32,
    value: u32,
    bit_count: u8,
}

#[cfg(test)]
impl<'a> BoolDecoder<'a> {
    pub(crate) fn new(input: &'a [u8]) -> Self {
        Self {
            input,
            position: 2,
            range: 255,
            value: (u32::from(input[0]) << 8) | u32::from(input[1]),
            bit_count: 0,
        }
    }

    pub(crate) fn read_bool(&mut self, probability: u8) -> bool {
        let split = 1 + (((self.range - 1) * u32::from(probability)) >> 8);
        let split_at_value_scale = split << 8;
        let value = self.value >= split_at_value_scale;

        if value {
            self.range -= split;
            self.value -= split_at_value_scale;
        } else {
            self.range = split;
        }

        while self.range < 128 {
            self.value <<= 1;
            self.range <<= 1;
            self.bit_count += 1;
            if self.bit_count == 8 {
                self.bit_count = 0;
                self.value |= u32::from(self.input[self.position]);
                self.position += 1;
            }
        }
        value
    }

    pub(crate) fn read_literal(&mut self, bit_count: u8) -> u32 {
        let mut value = 0;
        for _ in 0..bit_count {
            value = (value << 1) | u32::from(self.read_bool(128));
        }
        value
    }

    pub(crate) fn read_tree(&mut self, tree: &[i8], probabilities: &[u8], start_node: usize) -> u8 {
        let mut node = start_node;
        loop {
            let branch = usize::from(self.read_bool(probabilities[node >> 1]));
            let child = tree[node + branch];
            if child <= 0 {
                return child.unsigned_abs();
            }
            node = child as usize;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BoolDecoder, BoolEncoder};
    use crate::frame::{KF_UV_MODE_PROBS, KF_Y_MODE_PROBS, KF_Y_MODE_TREE, UV_MODE_TREE};
    use crate::residual::{COEFF_TREE, DEFAULT_COEFF_PROBS};

    #[derive(Clone, Copy)]
    enum Operation {
        Bool(u8, bool),
        Literal(u32, u8),
        Coefficient(u8),
        LumaMode(u8),
        ChromaMode(u8),
    }

    fn encode_pairs(pairs: &[(u8, bool)]) -> alloc::vec::Vec<u8> {
        let mut encoder = BoolEncoder::new();
        for &(probability, value) in pairs {
            encoder.write_bool(probability, value);
        }
        encoder.finish()
    }

    fn next_seed(seed: &mut u32) -> u32 {
        *seed ^= *seed << 13;
        *seed ^= *seed >> 17;
        *seed ^= *seed << 5;
        *seed
    }

    #[test]
    fn an_empty_partition_flushes_to_four_zero_bytes() {
        assert_eq!(BoolEncoder::new().finish(), [0, 0, 0, 0]);
    }

    #[test]
    fn the_eight_bit_literal_has_the_pinned_encoding() {
        let mut encoder = BoolEncoder::new();
        encoder.write_literal(0xa5, 8);
        assert_eq!(encoder.finish(), [0xa4, 0xb6, 0, 0]);
    }

    #[test]
    fn one_hundred_rare_ones_have_the_pinned_encoding() {
        assert_eq!(encode_pairs(&[(1, true); 100]), [0x64, 0, 0, 0]);
    }

    #[test]
    fn one_hundred_likely_zeros_have_the_pinned_encoding() {
        assert_eq!(encode_pairs(&[(255, false); 100]), [0, 0, 0, 0]);
    }

    #[test]
    fn the_mixed_probability_sequence_has_the_pinned_encoding() {
        let pairs: alloc::vec::Vec<_> = (0..64u32)
            .map(|index| {
                (
                    (1 + (37 * index) % 255) as u8,
                    (index * index + index / 3) % 2 != 0,
                )
            })
            .collect();
        assert_eq!(
            encode_pairs(&pairs),
            [0x00, 0x36, 0xa6, 0xa2, 0x13, 0xb8, 0x2c, 0x13, 0x14, 0xc5, 0x41, 0x0e, 0x80,]
        );
    }

    #[test]
    fn every_seeded_sequence_reads_back_at_each_length_through_four_thousand_ninety_six() {
        for length in 0..=4096usize {
            let mut seed = 0x9e37_79b9u32 ^ length as u32;
            let pairs: alloc::vec::Vec<_> = (0..length)
                .map(|_| {
                    let probability = (next_seed(&mut seed) % 255 + 1) as u8;
                    let value = next_seed(&mut seed) & 1 != 0;
                    (probability, value)
                })
                .collect();
            let encoded = encode_pairs(&pairs);
            let mut decoder = BoolDecoder::new(&encoded);
            let decoded: alloc::vec::Vec<_> = pairs
                .iter()
                .map(|&(probability, _)| decoder.read_bool(probability))
                .collect();
            let expected: alloc::vec::Vec<_> = pairs.iter().map(|&(_, value)| value).collect();
            assert_eq!(decoded, expected, "length {length}");
        }
    }

    #[test]
    fn literals_read_back_at_every_width_through_thirty_two_bits() {
        let mut encoder = BoolEncoder::new();
        let mut expected = alloc::vec::Vec::new();
        for bit_count in 0..=32u8 {
            let value = if bit_count == 0 {
                0
            } else {
                0xa5c3_7e91u32 >> (32 - u32::from(bit_count))
            };
            encoder.write_literal(value, bit_count);
            expected.push((value, bit_count));
        }
        let encoded = encoder.finish();
        let mut decoder = BoolDecoder::new(&encoded);
        for (value, bit_count) in expected {
            assert_eq!(decoder.read_literal(bit_count), value, "width {bit_count}");
        }
    }

    #[test]
    fn every_tree_leaf_reads_back_to_the_value_that_was_written() {
        let tree = [-2, 2, -0, 4, -1, -3];
        let probabilities = [37, 128, 241];
        let mut encoder = BoolEncoder::new();
        for value in [0, 1, 2, 3] {
            encoder.write_tree(&tree, &probabilities, value, 0);
        }
        let encoded = encoder.finish();
        let mut decoder = BoolDecoder::new(&encoded);
        for value in [0, 1, 2, 3] {
            assert_eq!(decoder.read_tree(&tree, &probabilities, 0), value);
        }
    }

    #[test]
    fn a_seeded_mix_of_bools_literals_and_tree_values_reads_back_exactly() {
        let mut seed = 0x91e1_0da5u32;
        let mut operations = alloc::vec::Vec::new();
        for probability in 1..=255u8 {
            operations.push(Operation::Bool(probability, next_seed(&mut seed) & 1 != 0));
            let width = (next_seed(&mut seed) % 8 + 1) as u8;
            let literal = next_seed(&mut seed) & ((1u32 << width) - 1);
            operations.push(Operation::Literal(literal, width));
            operations.push(Operation::Coefficient((next_seed(&mut seed) % 12) as u8));
            operations.push(Operation::LumaMode((next_seed(&mut seed) % 5) as u8));
            operations.push(Operation::ChromaMode((next_seed(&mut seed) % 4) as u8));
        }

        let mut encoder = BoolEncoder::new();
        for operation in &operations {
            match *operation {
                Operation::Bool(probability, value) => encoder.write_bool(probability, value),
                Operation::Literal(value, width) => encoder.write_literal(value, width),
                Operation::Coefficient(value) => {
                    encoder.write_tree(&COEFF_TREE, &DEFAULT_COEFF_PROBS[..11], value, 0)
                }
                Operation::LumaMode(value) => {
                    encoder.write_tree(&KF_Y_MODE_TREE, &KF_Y_MODE_PROBS, value, 0)
                }
                Operation::ChromaMode(value) => {
                    encoder.write_tree(&UV_MODE_TREE, &KF_UV_MODE_PROBS, value, 0)
                }
            }
        }

        let encoded = encoder.finish();
        let mut decoder = BoolDecoder::new(&encoded);
        for operation in operations {
            let matches = match operation {
                Operation::Bool(probability, value) => decoder.read_bool(probability) == value,
                Operation::Literal(value, width) => decoder.read_literal(width) == value,
                Operation::Coefficient(value) => {
                    decoder.read_tree(&COEFF_TREE, &DEFAULT_COEFF_PROBS[..11], 0) == value
                }
                Operation::LumaMode(value) => {
                    decoder.read_tree(&KF_Y_MODE_TREE, &KF_Y_MODE_PROBS, 0) == value
                }
                Operation::ChromaMode(value) => {
                    decoder.read_tree(&UV_MODE_TREE, &KF_UV_MODE_PROBS, 0) == value
                }
            };
            assert_eq!(u8::from(matches), 1);
        }
    }
}
