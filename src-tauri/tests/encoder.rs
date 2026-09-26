mod common;

use shrink_lib::engine::encoder::{detect, works, Encoder};

#[test]
fn detect_returns_the_first_encoder_that_works() {
    let t = common::tools();
    let chosen = detect(&t);
    assert!(works(&t, chosen), "{chosen:?} was chosen but does not encode");
    for e in Encoder::ORDER.iter().take_while(|e| **e != chosen) {
        assert!(!works(&t, *e), "{e:?} works but was skipped");
    }
}

#[test]
fn x264_always_works() {
    assert!(works(&common::tools(), Encoder::X264));
}
