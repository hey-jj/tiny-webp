//! The settings a caller gets before changing anything.

use tiny_webp::{Alpha, Filter, Options};

#[test]
fn the_default_options_are_quality_seventy_five_lossless_alpha_a_bare_container_and_the_auto_filter(
) {
    let opts = Options::default();
    assert_eq!(
        (opts.quality, opts.alpha, opts.force_vp8x, opts.filter),
        (75, Alpha::Lossless, false, Filter::Auto)
    );
}

#[test]
fn the_alpha_and_filter_defaults_stand_on_their_own() {
    assert_eq!(
        (Alpha::default(), Filter::default()),
        (Alpha::Lossless, Filter::Auto)
    );
}
