#![forbid(unsafe_code)]

use std::io::{self, Read, Write};

use tiny_webp::{Alpha, Error, Filter, Options};

#[path = "../../fixtures/generator.rs"]
mod generator;
pub(crate) use generator::Rng;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Input {
    pub(crate) pixels: Vec<u8>,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) options: Options,
}

pub(crate) fn corpus() -> Vec<Input> {
    let mut fixtures = generator::all();
    for (width, height) in [
        (16383, 1),
        (1, 16383),
        (16383, 3),
        (3, 16383),
        (4097, 1),
        (1, 4097),
    ] {
        fixtures.push(generator::noise("mutation-edge", width, height));
    }
    fixtures
        .into_iter()
        .map(|fixture| {
            let _name = fixture.name;
            Input {
                pixels: fixture.rgba,
                width: fixture.width,
                height: fixture.height,
                options: Options::default(),
            }
        })
        .collect()
}

pub(crate) fn outcome(input: &Input) -> Result<(), Error> {
    if !(1..=16383).contains(&input.width) || !(1..=16383).contains(&input.height) {
        return Err(Error::DimensionsOutOfRange {
            width: input.width,
            height: input.height,
        });
    }
    let expected = input.width as usize * input.height as usize * 4;
    if input.pixels.len() != expected {
        return Err(Error::BufferSizeMismatch {
            expected,
            actual: input.pixels.len(),
        });
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DimensionChange {
    Decrease,
    Increase,
    Double,
    One,
    Maximum,
    Zero,
    AboveMaximum,
    Swap,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mutation {
    FlipBit,
    SetByte,
    InsertByte,
    DeleteByte,
    Dimension {
        height: bool,
        change: DimensionChange,
    },
    Quality,
    Alpha,
    ForceVp8x,
    FilterAuto,
    FilterOff,
    FilterLevel,
}

impl Mutation {
    pub(crate) fn draw(rng: &mut Rng) -> Self {
        match rng.below(11) {
            0 => Self::FlipBit,
            1 => Self::SetByte,
            2 => Self::InsertByte,
            3 => Self::DeleteByte,
            4 => Self::Dimension {
                height: rng.below(2) == 1,
                change: match rng.below(8) {
                    0 => DimensionChange::Decrease,
                    1 => DimensionChange::Increase,
                    2 => DimensionChange::Double,
                    3 => DimensionChange::One,
                    4 => DimensionChange::Maximum,
                    5 => DimensionChange::Zero,
                    6 => DimensionChange::AboveMaximum,
                    _ => DimensionChange::Swap,
                },
            },
            5 => Self::Quality,
            6 => Self::Alpha,
            7 => Self::ForceVp8x,
            8 => Self::FilterAuto,
            9 => Self::FilterOff,
            _ => Self::FilterLevel,
        }
    }

    pub(crate) fn apply(self, input: &mut Input, rng: &mut Rng) {
        match self {
            Self::FlipBit | Self::SetByte | Self::DeleteByte if input.pixels.is_empty() => {}
            Self::FlipBit => {
                let index = rng.below(input.pixels.len() as u32) as usize;
                input.pixels[index] ^= 1 << rng.below(8);
            }
            Self::SetByte => {
                let index = rng.below(input.pixels.len() as u32) as usize;
                input.pixels[index] = rng.next_byte();
            }
            Self::InsertByte => {
                let index = rng.below(input.pixels.len() as u32 + 1) as usize;
                input.pixels.insert(index, rng.next_byte());
            }
            Self::DeleteByte => {
                let index = rng.below(input.pixels.len() as u32) as usize;
                input.pixels.remove(index);
            }
            Self::Dimension { height, change } => {
                if change == DimensionChange::Swap {
                    std::mem::swap(&mut input.width, &mut input.height);
                    return;
                }
                let side = if height {
                    &mut input.height
                } else {
                    &mut input.width
                };
                *side = match change {
                    DimensionChange::Decrease => side.saturating_sub(1),
                    DimensionChange::Increase => side.saturating_add(1),
                    DimensionChange::Double => side.saturating_mul(2),
                    DimensionChange::One => 1,
                    DimensionChange::Maximum => 16383,
                    DimensionChange::Zero => 0,
                    DimensionChange::AboveMaximum => 16384,
                    DimensionChange::Swap => unreachable!(),
                };
            }
            Self::Quality => input.options.quality = rng.next_byte(),
            Self::Alpha => {
                input.options.alpha = match input.options.alpha {
                    Alpha::Lossless => Alpha::Discard,
                    _ => Alpha::Lossless,
                };
            }
            Self::ForceVp8x => input.options.force_vp8x = !input.options.force_vp8x,
            Self::FilterAuto => input.options.filter = Filter::Auto,
            Self::FilterOff => input.options.filter = Filter::Off,
            Self::FilterLevel => {
                input.options.filter = Filter::Level {
                    level: rng.below(64) as u8,
                    sharpness: rng.below(8) as u8,
                };
            }
        }
    }
}

pub(crate) fn write_replay(input: &Input, mut output: impl Write) -> io::Result<()> {
    let alpha = match input.options.alpha {
        Alpha::Lossless => 0,
        Alpha::Discard => 1,
        _ => return Err(invalid_replay()),
    };
    let filter = match input.options.filter {
        Filter::Auto => [0, 0, 0],
        Filter::Off => [1, 0, 0],
        Filter::Level { level, sharpness } => [2, level, sharpness],
        _ => return Err(invalid_replay()),
    };
    output.write_all(b"TWM1")?;
    output.write_all(&input.width.to_le_bytes())?;
    output.write_all(&input.height.to_le_bytes())?;
    output.write_all(&[
        input.options.quality,
        alpha,
        u8::from(input.options.force_vp8x),
    ])?;
    output.write_all(&filter)?;
    output.write_all(&(input.pixels.len() as u64).to_le_bytes())?;
    output.write_all(&input.pixels)
}

pub(crate) fn read_replay(mut source: impl Read) -> io::Result<Input> {
    let mut header = [0; 26];
    source.read_exact(&mut header)?;
    if &header[..4] != b"TWM1" {
        return Err(invalid_replay());
    }
    let mut options = Options::default();
    options.quality = header[12];
    options.alpha = match header[13] {
        0 => Alpha::Lossless,
        1 => Alpha::Discard,
        _ => return Err(invalid_replay()),
    };
    options.force_vp8x = match header[14] {
        0 => false,
        1 => true,
        _ => return Err(invalid_replay()),
    };
    options.filter = match (header[15], header[16], header[17]) {
        (0, 0, 0) => Filter::Auto,
        (1, 0, 0) => Filter::Off,
        (2, level, sharpness) => Filter::Level { level, sharpness },
        _ => return Err(invalid_replay()),
    };
    let width = u32::from_le_bytes(header[4..8].try_into().unwrap());
    let height = u32::from_le_bytes(header[8..12].try_into().unwrap());
    let length = u64::from_le_bytes(header[18..26].try_into().unwrap());
    let mut pixels = Vec::new();
    source.read_to_end(&mut pixels)?;
    if pixels.len() as u64 != length {
        return Err(invalid_replay());
    }
    Ok(Input {
        pixels,
        width,
        height,
        options,
    })
}

fn invalid_replay() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "Invalid mutation replay record.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pinned_input() -> Input {
        Input {
            pixels: vec![0, 1, 2, 3, 4, 5, 6, 7],
            width: 2,
            height: 1,
            options: Options::default(),
        }
    }

    #[test]
    fn byte_mutations_change_the_drawn_position_to_the_pinned_value() {
        for (mutation, expected) in [
            (Mutation::FlipBit, vec![0, 1, 2, 11, 4, 5, 6, 7]),
            (Mutation::SetByte, vec![0, 1, 2, 107, 4, 5, 6, 7]),
            (Mutation::InsertByte, vec![0, 1, 2, 3, 107, 4, 5, 6, 7]),
            (Mutation::DeleteByte, vec![0, 1, 2, 4, 5, 6, 7]),
        ] {
            let mut input = pinned_input();
            mutation.apply(&mut input, &mut Rng::seeded("mutation-pins"));
            let mut result = pinned_input();
            result.pixels = expected;
            assert_eq!(input, result, "{mutation:?}");
        }
    }

    #[test]
    fn empty_pixels_accept_insertion_and_keep_other_byte_mutations_empty() {
        for mutation in [Mutation::FlipBit, Mutation::SetByte, Mutation::DeleteByte] {
            let mut input = pinned_input();
            input.pixels.clear();
            mutation.apply(&mut input, &mut Rng::seeded("mutation-pins"));
            assert_eq!(input.pixels, Vec::<u8>::new());
        }
        let mut input = pinned_input();
        input.pixels.clear();
        Mutation::InsertByte.apply(&mut input, &mut Rng::seeded("mutation-pins"));
        assert_eq!(input.pixels, [107]);
    }

    #[test]
    fn dimension_mutations_change_only_the_selected_side_or_swap_both() {
        for height in [false, true] {
            for (change, expected) in [
                (DimensionChange::Decrease, if height { 2 } else { 1 }),
                (DimensionChange::Increase, if height { 4 } else { 3 }),
                (DimensionChange::Double, if height { 6 } else { 4 }),
                (DimensionChange::One, 1),
                (DimensionChange::Maximum, 16383),
                (DimensionChange::Zero, 0),
                (DimensionChange::AboveMaximum, 16384),
                (DimensionChange::Swap, 0),
            ] {
                let mut input = pinned_input();
                input.height = 3;
                let mut result = input.clone();
                if change == DimensionChange::Swap {
                    result.width = 3;
                    result.height = 2;
                } else if height {
                    result.height = expected;
                } else {
                    result.width = expected;
                }
                Mutation::Dimension { height, change }
                    .apply(&mut input, &mut Rng::seeded("mutation-pins"));
                assert_eq!(input, result);
            }
        }
    }

    #[test]
    fn dimension_arithmetic_saturates_at_the_integer_boundaries() {
        for (start, change, expected) in [
            (0, DimensionChange::Decrease, 0),
            (u32::MAX, DimensionChange::Increase, u32::MAX),
            (u32::MAX, DimensionChange::Double, u32::MAX),
        ] {
            let mut input = pinned_input();
            input.width = start;
            Mutation::Dimension {
                height: false,
                change,
            }
            .apply(&mut input, &mut Rng::seeded("mutation-pins"));
            assert_eq!(input.width, expected);
        }
    }

    #[test]
    fn option_mutations_pin_quality_alpha_container_and_filter_values() {
        for mutation in [
            Mutation::Quality,
            Mutation::Alpha,
            Mutation::ForceVp8x,
            Mutation::FilterAuto,
            Mutation::FilterOff,
            Mutation::FilterLevel,
        ] {
            let mut input = pinned_input();
            input.options.filter = if mutation == Mutation::FilterOff {
                Filter::Auto
            } else {
                Filter::Off
            };
            let mut expected = input.clone();
            match mutation {
                Mutation::Quality => expected.options.quality = 127,
                Mutation::Alpha => expected.options.alpha = Alpha::Discard,
                Mutation::ForceVp8x => expected.options.force_vp8x = true,
                Mutation::FilterAuto => expected.options.filter = Filter::Auto,
                Mutation::FilterOff => expected.options.filter = Filter::Off,
                Mutation::FilterLevel => {
                    expected.options.filter = Filter::Level {
                        level: 31,
                        sharpness: 3,
                    }
                }
                _ => unreachable!(),
            }
            mutation.apply(&mut input, &mut Rng::seeded("mutation-pins"));
            assert_eq!(input, expected);
        }
        let mut input = pinned_input();
        for mutation in [Mutation::Alpha, Mutation::ForceVp8x] {
            for _ in 0..2 {
                mutation.apply(&mut input, &mut Rng::seeded("mutation-pins"));
            }
        }
        assert_eq!(input, pinned_input());
    }

    #[test]
    fn named_draws_reach_every_mutation_and_filter_level_and_sharpness_pair() {
        let mut rng = Rng::seeded("mutation-pins");
        assert_eq!(Mutation::draw(&mut rng), Mutation::Quality);
        let mut mutations = Vec::new();
        for _ in 0..10_000 {
            let mutation = Mutation::draw(&mut rng);
            if !mutations.contains(&mutation) {
                mutations.push(mutation);
            }
        }
        assert_eq!(mutations.len(), 26);
        let mut seen = [[false; 8]; 64];
        let mut input = pinned_input();
        for _ in 0..10_000 {
            Mutation::FilterLevel.apply(&mut input, &mut rng);
            if let Filter::Level { level, sharpness } = input.options.filter {
                seen[level as usize][sharpness as usize] = true;
            }
        }
        assert_eq!(seen, [[true; 8]; 64]);
    }

    #[test]
    fn the_corpus_preserves_every_fixture_and_adds_the_six_edge_dimensions() {
        let inputs = corpus();
        let fixtures = generator::all();
        assert_eq!(inputs.len(), 21);
        for (input, fixture) in inputs.iter().zip(&fixtures) {
            assert_eq!(input.pixels, fixture.rgba);
            assert_eq!((input.width, input.height), (fixture.width, fixture.height));
            assert_eq!(input.options, Options::default());
        }
        let dimensions: Vec<_> = inputs[15..]
            .iter()
            .map(|input| (input.width, input.height))
            .collect();
        assert_eq!(
            dimensions,
            [
                (16383, 1),
                (1, 16383),
                (16383, 3),
                (3, 16383),
                (4097, 1),
                (1, 4097)
            ]
        );
        assert_eq!(inputs, corpus());
    }

    #[test]
    fn the_expected_outcome_agrees_with_the_encoder_on_every_corpus_entry() {
        for input in corpus() {
            assert_eq!(outcome(&input), Ok(()));
            assert_eq!(
                tiny_webp::encode_rgba(&input.pixels, input.width, input.height, &input.options)
                    .map(|_| ()),
                outcome(&input)
            );
        }
    }

    #[test]
    fn invalid_dimensions_take_precedence_over_short_and_long_buffers() {
        for (width, height, length, expected) in [
            (
                0,
                1,
                0,
                Err(Error::DimensionsOutOfRange {
                    width: 0,
                    height: 1,
                }),
            ),
            (
                1,
                16384,
                9,
                Err(Error::DimensionsOutOfRange {
                    width: 1,
                    height: 16384,
                }),
            ),
            (
                u32::MAX,
                u32::MAX,
                8,
                Err(Error::DimensionsOutOfRange {
                    width: u32::MAX,
                    height: u32::MAX,
                }),
            ),
            (
                2,
                1,
                7,
                Err(Error::BufferSizeMismatch {
                    expected: 8,
                    actual: 7,
                }),
            ),
            (
                2,
                1,
                9,
                Err(Error::BufferSizeMismatch {
                    expected: 8,
                    actual: 9,
                }),
            ),
        ] {
            let mut input = pinned_input();
            input.width = width;
            input.height = height;
            input.pixels.resize(length, 0);
            assert_eq!(outcome(&input), expected);
            assert_eq!(
                tiny_webp::encode_rgba(&input.pixels, width, height, &input.options).map(|_| ()),
                expected
            );
        }
    }

    #[test]
    fn replay_bytes_pin_the_header_options_length_and_pixels() {
        let mut input = pinned_input();
        input.width = 0x12345678;
        input.height = 0x90abcdef;
        input.options.quality = 255;
        input.options.alpha = Alpha::Discard;
        input.options.force_vp8x = true;
        input.options.filter = Filter::Level {
            level: 255,
            sharpness: 254,
        };
        let mut bytes = Vec::new();
        write_replay(&input, &mut bytes).unwrap();
        assert_eq!(
            bytes,
            [
                84, 87, 77, 49, 120, 86, 52, 18, 239, 205, 171, 144, 255, 1, 1, 2, 255, 254, 8, 0,
                0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7
            ]
        );
        assert_eq!(read_replay(bytes.as_slice()).unwrap(), input);
        for filter in [Filter::Auto, Filter::Off] {
            input.options.filter = filter;
            input.pixels.clear();
            bytes.clear();
            write_replay(&input, &mut bytes).unwrap();
            assert_eq!(read_replay(bytes.as_slice()).unwrap(), input);
        }
    }

    #[test]
    fn every_corpus_entry_survives_a_replay_file_byte_for_byte() {
        let directory = std::env::temp_dir().join(format!(
            "tiny-webp-every_corpus_entry_survives_a_replay_file_byte_for_byte-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        for (index, input) in corpus().into_iter().enumerate() {
            let path = directory.join(format!("{index}.replay"));
            write_replay(&input, std::fs::File::create(&path).unwrap()).unwrap();
            let restored = read_replay(std::fs::File::open(&path).unwrap()).unwrap();
            assert_eq!(restored, input);
            let mut bytes = Vec::new();
            write_replay(&restored, &mut bytes).unwrap();
            assert_eq!(bytes, std::fs::read(path).unwrap());
        }
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn replay_reads_reject_invalid_headers_and_inexact_payload_lengths() {
        let mut bytes = Vec::new();
        write_replay(&pinned_input(), &mut bytes).unwrap();
        for (index, value) in [(0, 0), (13, 2), (14, 2), (15, 3), (16, 1), (17, 1), (18, 9)] {
            let mut invalid = bytes.clone();
            invalid[index] = value;
            assert_eq!(
                read_replay(invalid.as_slice()).unwrap_err().kind(),
                io::ErrorKind::InvalidData
            );
        }
        for length in 0..bytes.len() {
            let expected = if length < 26 {
                io::ErrorKind::UnexpectedEof
            } else {
                io::ErrorKind::InvalidData
            };
            assert_eq!(read_replay(&bytes[..length]).unwrap_err().kind(), expected);
        }
        bytes.push(0);
        assert_eq!(
            read_replay(bytes.as_slice()).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }
}
