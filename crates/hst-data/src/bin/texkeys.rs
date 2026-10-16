//! texkeys <iso>  — every disc texture's key and `--dump-textures` name (`KEY<TAB>ARCHIVE/entry/path.N`), for
//! `research/assets_used.py` (Z2) to tie the asset-use log's `tex-draw KEY` lines back to the entries they came from.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut iso = hst_data::iso::Iso::open(std::env::args().nth(1).ok_or("usage: texkeys <iso>")?)?;
    hst_data::texhash::disc_textures(&mut iso, |t| println!("{}\t{}", t.key, t.path));
    Ok(())
}
