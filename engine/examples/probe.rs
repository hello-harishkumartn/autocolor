//! Calibration probe: how each LightCraft slider moves mean L*, spread and chroma on a fixture.
use autocolor_engine::render::{Path, render};
use lightcraft_develop::DevelopSettings;

fn main() {
    let name = std::env::args().nth(1).unwrap_or("city".into());
    let im = image::open(format!("../tests/fixtures/{name}.jpg")).unwrap().to_rgba8();
    let (w, h) = (im.width() as usize, im.height() as usize);
    let base = im.into_raw();
    let st = autocolor_engine::stats::measure(&base, w, h);
    println!("{st:?}");
    let probe = |label: &str, f: &dyn Fn(&mut DevelopSettings)| {
        let mut s = DevelopSettings::default();
        f(&mut s);
        let mut out = base.clone();
        render(&mut out, w, h, &s, Path::Full);
        let m = autocolor_engine::stats::measure(&out, w, h);
        println!("{label:>14}: p05 {:.3} p50 {:.3} p95 {:.3} p995 {:.3} chroma {:.4} ab {:?} clip {:.4}", m.p05, m.p50, m.p95, m.p995, m.chroma, m.grey_ab, m.clip_hi);
    };
    probe("none", &|_| {});
    for ev in [-1.0, 0.5, 1.0, 2.0] { probe(&format!("exp {ev}"), &|s| s.light.exposure = ev); }
    for v in [-50.0, 50.0] { probe(&format!("contrast {v}"), &|s| s.light.contrast = v); }
    for v in [-50.0, 50.0] { probe(&format!("highl {v}"), &|s| s.light.highlights = v); }
    for v in [-50.0, 50.0] { probe(&format!("shadows {v}"), &|s| s.light.shadows = v); }
    for v in [-50.0, 50.0] { probe(&format!("whites {v}"), &|s| s.light.whites = v); }
    for v in [-50.0, 50.0] { probe(&format!("blacks {v}"), &|s| s.light.blacks = v); }
    for v in [30.0] { probe(&format!("vibrance {v}"), &|s| s.color.vibrance = v); }
    for (t, ti) in [(5500.0, 0.0), (7500.0, 0.0), (6500.0, 20.0)] {
        probe(&format!("wb {t} {ti}"), &|s| { s.wb.mode = lightcraft_develop::WbMode::Custom; s.wb.temp = t; s.wb.tint = ti; });
    }
}
