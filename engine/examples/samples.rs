//! Writes before and after samples for the report into ../samples/ (and test inputs for the
//! browser checks). Run: cargo run --release --example samples
use autocolor_engine::{correct_rgba8, render::Path};
#[path = "../tests/common.rs"]
mod common;
#[path = "../tests/degrade.rs"]
mod degrade;

fn save_jpg(path: &str, rgba: &[u8], w: usize, h: usize) {
    let rgb: Vec<u8> = rgba.chunks(4).flat_map(|p| [p[0], p[1], p[2]]).collect();
    image::RgbImage::from_raw(w as u32, h as u32, rgb).unwrap().save_with_format(path, image::ImageFormat::Jpeg).unwrap();
}

fn main() {
    let out = format!("{}/../samples", env!("CARGO_MANIFEST_DIR"));
    std::fs::create_dir_all(&out).unwrap();
    let cases = [("lowlight", "people", "noisy"), ("dark", "city", "dark"), ("cast", "food", "warm"), ("green", "beach", "green"), ("flat", "landscape", "flat"), ("overexposed", "flowers", "over"), ("good", "fog", "none")];
    let mut rows = Vec::new();
    for (label, name, kind) in cases {
        let img = common::fixture(name);
        let before = if kind == "none" { img.rgba.clone() } else { degrade::degrade(&img.rgba, kind) };
        let mut after = before.clone();
        let r = correct_rgba8(&mut after, img.w, img.h, Path::Full);
        save_jpg(&format!("{out}/{label}_before.jpg"), &before, img.w, img.h);
        save_jpg(&format!("{out}/{label}_after.jpg"), &after, img.w, img.h);
        println!("{label:>12} ({name}, {kind}): dE00 to original {:.2} -> {:.2}  {:?}", common::mean_de(&img.rgba, &before), common::mean_de(&img.rgba, &after), r.choice);
        rows.push((before, after, img.w, img.h));
    }
    // side by side: before | after per row, each scaled to 170 px high
    let th = 170usize;
    let scaled: Vec<(image::RgbaImage, image::RgbaImage)> = rows
        .iter()
        .map(|(b, a, w, h)| {
            let tw = (*w * th / *h) as u32;
            let f = |v: &Vec<u8>| image::imageops::resize(&image::RgbaImage::from_raw(*w as u32, *h as u32, v.clone()).unwrap(), tw, th as u32, image::imageops::FilterType::Triangle);
            (f(b), f(a))
        })
        .collect();
    let width = scaled.iter().map(|(b, _)| b.width() * 2 + 12).max().unwrap();
    let height = (th as u32 + 8) * scaled.len() as u32;
    let mut sheet = image::RgbaImage::from_pixel(width, height, image::Rgba([13, 11, 9, 255]));
    for (i, (b, a)) in scaled.iter().enumerate() {
        let y = i as i64 * (th as i64 + 8);
        image::imageops::overlay(&mut sheet, b, 0, y);
        image::imageops::overlay(&mut sheet, a, b.width() as i64 + 12, y);
    }
    sheet.save(format!("{out}/compare.png")).unwrap();

    // a 12.6 MP input for the browser check of the banded path
    let big = image::imageops::resize(&image::RgbaImage::from_raw(rows[1].2 as u32, rows[1].3 as u32, rows[1].0.clone()).unwrap(), 2900, 4350, image::imageops::FilterType::Triangle);
    save_jpg("/tmp/ab-inputs-big.jpg", big.as_raw(), 2900, 4350);
}
