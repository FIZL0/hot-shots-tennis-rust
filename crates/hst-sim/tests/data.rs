//! The tests here skip quietly when the disc or the recordings are missing (they're never in git). A run that has to
//! check the port against the game sets `HST_REQUIRE_DATA=1`, and this fails it when the data isn't there.
// ponytail: checks the folders, not each fixture; a missing single recording still skips

#[test]
fn the_game_data_is_here_when_required() {
    if std::env::var_os("HST_REQUIRE_DATA").is_none() {
        return;
    }
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    for dir in ["context/iso/ZZBIN/GAME.BIN", "context/ram", "context/xb"] {
        assert!(std::path::Path::new(root).join(dir).exists(), "HST_REQUIRE_DATA: no {dir}");
    }
}
