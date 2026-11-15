//! Prints the measurements of every fixture (used to set the target bands).
fn main() {
    for name in ["landscape", "city", "food", "flowers", "beach", "people", "fog"] {
        let im = image::open(format!("../tests/fixtures/{name}.jpg")).unwrap().to_rgba8();
        let (w, h) = (im.width() as usize, im.height() as usize);
        let s = autocolor_engine::stats::measure(&im.into_raw(), w, h);
        println!("{name:>9} p005 {:.3} p05 {:.3} p50 {:.3} p95 {:.3} p995 {:.3} logm {:.2} chroma {:.3} grey {:+.4} {:+.4} white {:+.4} {:+.4} dom {:.2} noise {:.2} clip {:.4}",
            s.p005, s.p05, s.p50, s.p95, s.p995, s.log_mean, s.chroma, s.grey_ab[0], s.grey_ab[1], s.white_ab[0], s.white_ab[1], s.dominant_hue, s.noise, s.clip_hi);
    }
}
