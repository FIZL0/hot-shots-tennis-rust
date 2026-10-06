//! The round1 fixture (`context/fixtures/round1.bin`, not in git; skipped when absent) is a frame-exact capture:
//! it starts on the recording's save-state frame and has one sample per vsync with no gaps, and the decoded pad
//! the game read shows P1's inputs (the recording drives port 0; the bots' port 1 stays idle).

use hst_sim::replay::frames;

#[test]
fn round1_fixture_is_frame_exact() {
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let Ok(data) = std::fs::read(format!("{dir}/round1.bin")) else { return eprintln!("round1.bin absent, skipped") };
    let frames = frames(&data);
    assert_eq!(frames[0].vsync(), 10018, "round1_SaveState.p2s frame");
    for w in frames.windows(2) {
        assert_eq!(w[1].vsync(), w[0].vsync() + 1, "gap after vsync {}", w[0].vsync());
    }
    let pressed = frames.iter().filter(|f| f.pad(0).buttons != 0).count();
    let moved = frames.iter().filter(|f| (f.pad(0).lx, f.pad(0).ly) != (0x80, 0x80)).count();
    assert!(pressed > 0 && moved > 0, "no P1 input in the capture");
    eprintln!("round1: {} frames, P1 pressed on {pressed}, stick on {moved}", frames.len());
}
