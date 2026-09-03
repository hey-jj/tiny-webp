//! The text and the trait an `Error` carries.

use tiny_webp::Error;

#[test]
fn an_error_coerces_to_a_boxed_standard_error() {
    let boxed: Box<dyn core::error::Error> = Box::new(Error::DimensionsOutOfRange {
        width: 0,
        height: 4,
    });
    assert_eq!(boxed.to_string(), "dimensions 0x4 fall outside 1..=16383");
}

#[test]
fn the_dimensions_text_names_both_sides_and_the_range() {
    assert_eq!(
        Error::DimensionsOutOfRange {
            width: 16384,
            height: 7
        }
        .to_string(),
        "dimensions 16384x7 fall outside 1..=16383"
    );
}

#[test]
fn the_buffer_text_names_the_count_that_arrived_and_the_count_the_dimensions_ask_for() {
    assert_eq!(
        Error::BufferSizeMismatch {
            expected: 80,
            actual: 79
        }
        .to_string(),
        "buffer holds 79 bytes and the dimensions ask for 80"
    );
}
