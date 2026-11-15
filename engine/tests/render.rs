//! The LightCraft pipeline adapter: RGBA8 in, RGBA8 out.
mod common;
use autocolor_engine::render::{Path, render};
use common::*;
use lightcraft_develop::DevelopSettings;

#[test]
fn default_settings_leave_a_jpeg_unchanged() {
    for name in ["city", "people"] {
        let img = fixture(name);
        let mut out = img.rgba.clone();
        render(&mut out, img.w, img.h, &DevelopSettings::default(), Path::Full);
        let max = img.rgba.iter().zip(&out).map(|(a, b)| (*a as i32 - *b as i32).abs()).max().unwrap();
        assert!(max <= 1, "{name}: max difference {max}");
    }
}

#[path = "degrade.rs"]
mod degrade;

/// A fixture enlarged 3x so the banded path really works in bands and upsamples its gain.
fn large(name: &str) -> Img {
    let im = image::open(format!("{}/../tests/fixtures/{name}.jpg", env!("CARGO_MANIFEST_DIR"))).unwrap();
    let im = im.resize(im.width() * 3, im.height() * 3, image::imageops::FilterType::Triangle).to_rgba8();
    Img { w: im.width() as usize, h: im.height() as usize, rgba: im.into_raw() }
}

#[test]
fn the_banded_path_matches_the_full_render() {
    for (name, kind) in [("city", "dark"), ("people", "noisy"), ("landscape", "flat"), ("food", "warm")] {
        let img = large(name);
        let bad = degrade::degrade(&img.rgba, kind);
        let (mut full, mut banded) = (bad.clone(), bad.clone());
        let r1 = autocolor_engine::correct_rgba8(&mut full, img.w, img.h, Path::Full);
        let r2 = autocolor_engine::correct_rgba8(&mut banded, img.w, img.h, Path::Banded);
        assert_eq!(r1.choice, r2.choice);
        let de = mean_de(&full, &banded);
        println!("{name} {kind}: banded vs full dE00 {de:.2} ({:?})", r1.choice);
        assert!(de < 2.0, "{name} {kind}: banded differs from full by {de:.2}");
        assert!(mean_de(&img.rgba, &banded) < mean_de(&img.rgba, &bad), "{name} {kind}: banded did not improve");
    }
}
