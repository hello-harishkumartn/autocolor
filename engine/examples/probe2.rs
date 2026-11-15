//! Calibration probe 2: which settings bring a darkened fixture back closest to the original.
use autocolor_engine::render::{Path, render};
use lightcraft_develop::DevelopSettings;
#[path = "../tests/common.rs"]
mod common;

fn main() {
    let name = std::env::args().nth(1).unwrap_or("city".into());
    let gain: f32 = std::env::args().nth(2).map(|s| s.parse().unwrap()).unwrap_or(0.25);
    let img = common::fixture(&name);
    let lin = |v: u8| { let c = v as f32 / 255.0; if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) } };
    let enc = |l: f32| { let l = l.clamp(0.0, 1.0); (if l <= 0.0031308 { l * 12.92 } else { 1.055 * l.powf(1.0 / 2.4) - 0.055 } * 255.0).round() as u8 };
    let dark: Vec<u8> = img.rgba.chunks(4).flat_map(|p| [enc(lin(p[0]) * gain), enc(lin(p[1]) * gain), enc(lin(p[2]) * gain), 255]).collect();
    let st = autocolor_engine::stats::measure(&dark, img.w, img.h);
    println!("degraded: dE {:.2} L {:.1} (orig L {:.1}) p50 {:.3} p995 {:.3}", common::mean_de(&img.rgba, &dark), common::mean_l(&dark), common::mean_l(&img.rgba), st.p50, st.p995);
    let ev0 = -gain.log2();
    for (ev, hl, wh, sh) in [(ev0, 0.0, 0.0, 0.0), (ev0, -100.0, 0.0, 0.0), (ev0 * 0.8, -60.0, 0.0, 30.0), (ev0, -60.0, -30.0, 0.0), (ev0 * 0.7, 0.0, 0.0, 60.0)] {
        let mut s = DevelopSettings::default();
        s.light.exposure = ev as f64; s.light.highlights = hl; s.light.whites = wh; s.light.shadows = sh;
        let mut out = dark.clone();
        render(&mut out, img.w, img.h, &s, Path::Full);
        println!("ev {ev:.2} hl {hl} wh {wh} sh {sh}: dE {:.2} L {:.1} clip {:.4}", common::mean_de(&img.rgba, &out), common::mean_l(&out), common::clipped(&out));
    }
}
