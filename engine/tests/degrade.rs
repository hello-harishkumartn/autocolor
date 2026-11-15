#![allow(dead_code)]
//! Synthetic degradations of the fixtures (applied in linear light where physical).

fn lin(v: u8) -> f32 {
    let c = v as f32 / 255.0;
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}
fn enc(l: f32) -> u8 {
    let l = l.clamp(0.0, 1.0);
    ((if l <= 0.0031308 { l * 12.92 } else { 1.055 * l.powf(1.0 / 2.4) - 0.055 }) * 255.0).round() as u8
}
fn gains(src: &[u8], g: [f32; 3]) -> Vec<u8> {
    src.chunks(4).flat_map(|p| [enc(lin(p[0]) * g[0]), enc(lin(p[1]) * g[1]), enc(lin(p[2]) * g[2]), 255]).collect()
}

pub const KINDS: [&str; 7] = ["dark", "over", "warm", "cool", "green", "flat", "noisy"];

pub fn degrade(src: &[u8], kind: &str) -> Vec<u8> {
    match kind {
        "dark" => gains(src, [0.25; 3]),
        "over" => gains(src, [2.2; 3]),
        "warm" => gains(src, [1.12, 1.0, 0.78]),
        "cool" => gains(src, [0.8, 0.97, 1.15]),
        "green" => gains(src, [0.9, 1.1, 0.9]),
        "flat" => src.chunks(4).flat_map(|p| [0, 1, 2].map(|c| (128.0 + (p[c] as f32 - 128.0) * 0.6).round() as u8).into_iter().chain([255])).collect(),
        "noisy" => {
            // a 1.5 EV underexposure with sensor-like noise added before the photo was brightened back
            let mut s: u64 = 99;
            let mut rnd = || {
                s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                ((s >> 11) as f64 + 0.5) / (1u64 << 53) as f64
            };
            let dark = gains(src, [0.35; 3]);
            dark.chunks(4)
                .flat_map(|p| {
                    let n = ((-2.0 * rnd().ln()).sqrt() * (2.0 * std::f64::consts::PI * rnd()).cos() * 5.0) as f32;
                    [0, 1, 2].map(|c| (p[c] as f32 + n).round().clamp(0.0, 255.0) as u8).into_iter().chain([255])
                })
                .collect()
        }
        _ => unreachable!("{kind}"),
    }
}
