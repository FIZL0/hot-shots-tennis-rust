# Modding

The character mod standard, its skeleton, the tutorial and every tool for extracting characters and making mods live
in the mods repo: <https://github.com/FIZL0/hot-shots-tennis-rust-mods> (`standard/`, `tools/`).

The game loads mods from `mods/<id>/` beside the ISO (`crates/hst/src/mods.rs`). It carries a copy of the skeleton,
`crates/hst/hst_skeleton.json`, to check models against; keep it equal to the mods repo's `standard/hst_skeleton.json`.
`tools/upscale_mods.py` upscales mod textures (needs this repo's chaiNNer chain).
