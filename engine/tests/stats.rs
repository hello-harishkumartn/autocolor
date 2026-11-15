//! Photo measurements on synthetic images with known properties.
mod common;
use autocolor_engine::stats::measure;
use common::*;

fn filled(w: usize, h: usize, f: impl Fn(usize, usize) -> [u8; 3]) -> Vec<u8> {
    let mut v = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for x in 0..w {
            let c = f(x, y);
            v.extend_from_slice(&[c[0], c[1], c[2], 255]);
        }
    }
    v
}

/// Deterministic Gaussian noise (Box-Muller over a small LCG).
fn noisy(base: &[u8], sigma: f64) -> Vec<u8> {
    let mut s: u64 = 12345;
    let mut rnd = || {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((s >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    };
    base.chunks(4)
        .flat_map(|p| {
            let n = (-2.0 * rnd().ln()).sqrt() * (2.0 * std::f64::consts::PI * rnd()).cos() * sigma;
            [0, 1, 2].map(|c| (p[c] as f64 + n).round().clamp(0.0, 255.0) as u8).into_iter().chain([255])
        })
        .collect()
}

#[test]
fn a_mid_grey_image_measures_mid_grey_without_cast_or_noise() {
    let img = filled(64, 48, |_, _| [119, 119, 119]);
    let s = measure(&img, 64, 48);
    assert!((s.p50 - 119.0 / 255.0).abs() < 0.01, "{s:?}");
    assert!(s.p005 <= s.p05 && s.p05 <= s.p50 && s.p50 <= s.p95 && s.p95 <= s.p995);
    assert!(s.grey_ab[0].abs() < 0.002 && s.grey_ab[1].abs() < 0.002, "{s:?}");
    assert!(s.noise < 0.5, "{s:?}");
    assert_eq!((s.clip_hi, s.clip_lo), (0.0, 0.0));
}

#[test]
fn percentiles_follow_a_ramp() {
    let img = filled(256, 16, |x, _| [x as u8; 3]);
    let s = measure(&img, 256, 16);
    assert!((s.p50 - 0.5).abs() < 0.03, "{s:?}");
    assert!((s.p05 - 0.05).abs() < 0.03 && (s.p95 - 0.95).abs() < 0.03, "{s:?}");
    assert!(s.clip_hi > 0.0 && s.clip_lo > 0.0);
}

#[test]
fn a_warm_cast_shows_as_yellow_red_in_the_grey_world_estimate() {
    let img = fixture("city");
    let warm: Vec<u8> = img.rgba.chunks(4).flat_map(|p| [(p[0] as f32 * 1.12).min(255.0) as u8, p[1], (p[2] as f32 * 0.8) as u8, 255]).collect();
    let (a, b) = (measure(&img.rgba, img.w, img.h), measure(&warm, img.w, img.h));
    assert!(b.grey_ab[1] > a.grey_ab[1] + 0.01, "b: {:?} vs {:?}", b.grey_ab, a.grey_ab);
    assert!(b.grey_ab[0] > a.grey_ab[0], "a: {:?} vs {:?}", b.grey_ab, a.grey_ab);
    assert!(b.white_ab[1] > a.white_ab[1], "white patch b: {:?} vs {:?}", b.white_ab, a.white_ab);
}

#[test]
fn noise_is_measured_in_8_bit_levels() {
    let smooth = filled(600, 400, |x, y| [(60 + x / 8) as u8, (70 + y / 8) as u8, 90]);
    let clean = measure(&smooth, 600, 400).noise;
    let n8 = measure(&noisy(&smooth, 8.0), 600, 400).noise;
    assert!(clean < 1.0, "clean {clean}");
    assert!((6.0..10.0).contains(&n8), "sigma 8 measured {n8}");
    // a real photo's texture is not mistaken for heavy noise
    let city = fixture("city");
    assert!(measure(&city.rgba, city.w, city.h).noise < 4.0);
}

#[test]
fn chroma_and_dominant_hue() {
    let grey = filled(64, 64, |x, _| [x as u8 * 3; 3]);
    let red = filled(64, 64, |x, _| [120 + x as u8, 30, 30]);
    let (g, r) = (measure(&grey, 64, 64), measure(&red, 64, 64));
    assert!(g.chroma < 0.005 && r.chroma > 0.1, "{} {}", g.chroma, r.chroma);
    assert!(r.dominant_hue > 0.9, "{}", r.dominant_hue);
}
