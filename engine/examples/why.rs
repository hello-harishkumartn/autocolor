//! Prints measurements and choices for a fixture under one degradation (tuning aid).
#[path = "../tests/common.rs"]
mod common;
#[path = "../tests/degrade.rs"]
mod degrade;
fn main() {
    let kind = std::env::args().nth(1).unwrap_or("warm".into());
    for name in common::FIXTURES {
        let img = common::fixture(name);
        let bad = if kind == "none" { img.rgba.clone() } else { degrade::degrade(&img.rgba, &kind) };
        let s = autocolor_engine::stats::measure(&bad, img.w, img.h);
        let c = autocolor_engine::params::choose(&s);
        println!("{name:>9} grey {:+.4} {:+.4} white {:+.4} {:+.4} dom {:.2} wb {:?} -> str {:.2} temp {} tint {} | ev {} sh {} hl {} b {} w {} vib {} nr {}",
            s.grey_ab[0], s.grey_ab[1], s.white_ab[0], s.white_ab[1], s.dominant_hue, s.auto_wb, c.wb_strength, c.temp, c.tint, c.exposure, c.shadows, c.highlights, c.black, c.white, c.vibrance, c.nr_luminance);
    }
}
