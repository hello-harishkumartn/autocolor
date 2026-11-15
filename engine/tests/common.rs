#![allow(dead_code)]
//! Shared helpers for the integration tests: fixture loading and colour metrics.

pub const FIXTURES: [&str; 7] = ["landscape", "city", "food", "flowers", "beach", "people", "fog"];

pub struct Img {
    pub w: usize,
    pub h: usize,
    pub rgba: Vec<u8>,
}

pub fn fixture(name: &str) -> Img {
    let path = format!("{}/../tests/fixtures/{name}.jpg", env!("CARGO_MANIFEST_DIR"));
    let im = image::open(&path).unwrap_or_else(|e| panic!("{path}: {e}")).to_rgba8();
    Img { w: im.width() as usize, h: im.height() as usize, rgba: im.into_raw() }
}

fn lin(v: u8) -> f64 {
    let c = v as f64 / 255.0;
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

/// CIE L*a*b* (D65) of an sRGB8 pixel.
pub fn lab(p: &[u8]) -> [f64; 3] {
    let (r, g, b) = (lin(p[0]), lin(p[1]), lin(p[2]));
    let x = (0.4124 * r + 0.3576 * g + 0.1805 * b) / 0.95047;
    let y = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    let z = (0.0193 * r + 0.1192 * g + 0.9505 * b) / 1.08883;
    let f = |t: f64| if t > 0.008856 { t.cbrt() } else { 7.787 * t + 16.0 / 116.0 };
    let (fx, fy, fz) = (f(x), f(y), f(z));
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

/// CIEDE2000 colour difference.
pub fn de2000(a: [f64; 3], b: [f64; 3]) -> f64 {
    let (l1, a1, b1) = (a[0], a[1], a[2]);
    let (l2, a2, b2) = (b[0], b[1], b[2]);
    let c1 = (a1 * a1 + b1 * b1).sqrt();
    let c2 = (a2 * a2 + b2 * b2).sqrt();
    let cm = (c1 + c2) / 2.0;
    let g = 0.5 * (1.0 - (cm.powi(7) / (cm.powi(7) + 25f64.powi(7))).sqrt());
    let (a1p, a2p) = ((1.0 + g) * a1, (1.0 + g) * a2);
    let (c1p, c2p) = ((a1p * a1p + b1 * b1).sqrt(), (a2p * a2p + b2 * b2).sqrt());
    let h = |b: f64, a: f64| if a == 0.0 && b == 0.0 { 0.0 } else { let d = b.atan2(a).to_degrees(); if d < 0.0 { d + 360.0 } else { d } };
    let (h1, h2) = (h(b1, a1p), h(b2, a2p));
    let dl = l2 - l1;
    let dc = c2p - c1p;
    let mut dh = h2 - h1;
    if c1p * c2p != 0.0 {
        if dh > 180.0 { dh -= 360.0 } else if dh < -180.0 { dh += 360.0 }
    } else {
        dh = 0.0
    }
    let dhh = 2.0 * (c1p * c2p).sqrt() * (dh.to_radians() / 2.0).sin();
    let lm = (l1 + l2) / 2.0;
    let cmp = (c1p + c2p) / 2.0;
    let hm = if c1p * c2p == 0.0 {
        h1 + h2
    } else if (h1 - h2).abs() <= 180.0 {
        (h1 + h2) / 2.0
    } else if h1 + h2 < 360.0 {
        (h1 + h2 + 360.0) / 2.0
    } else {
        (h1 + h2 - 360.0) / 2.0
    };
    let t = 1.0 - 0.17 * (hm - 30.0).to_radians().cos() + 0.24 * (2.0 * hm).to_radians().cos() + 0.32 * (3.0 * hm + 6.0).to_radians().cos()
        - 0.20 * (4.0 * hm - 63.0).to_radians().cos();
    let sl = 1.0 + 0.015 * (lm - 50.0).powi(2) / (20.0 + (lm - 50.0).powi(2)).sqrt();
    let sc = 1.0 + 0.045 * cmp;
    let sh = 1.0 + 0.015 * cmp * t;
    let dtheta = 30.0 * (-((hm - 275.0) / 25.0).powi(2)).exp();
    let rc = 2.0 * (cmp.powi(7) / (cmp.powi(7) + 25f64.powi(7))).sqrt();
    let rt = -(2.0 * dtheta.to_radians()).sin() * rc;
    ((dl / sl).powi(2) + (dc / sc).powi(2) + (dhh / sh).powi(2) + rt * (dc / sc) * (dhh / sh)).sqrt()
}

/// Mean CIEDE2000 between two RGBA8 images of the same size.
pub fn mean_de(a: &[u8], b: &[u8]) -> f64 {
    assert_eq!(a.len(), b.len());
    let n = a.len() / 4;
    a.chunks(4).zip(b.chunks(4)).map(|(p, q)| de2000(lab(p), lab(q))).sum::<f64>() / n as f64
}

/// Mean L* of an RGBA8 image.
pub fn mean_l(a: &[u8]) -> f64 {
    a.chunks(4).map(|p| lab(p)[0]).sum::<f64>() / (a.len() / 4) as f64
}

/// Colour cast size: length of the mean (a*, b*) over the image.
pub fn cast(a: &[u8]) -> f64 {
    let n = (a.len() / 4) as f64;
    let (sa, sb) = a.chunks(4).map(|p| lab(p)).fold((0.0, 0.0), |(x, y), l| (x + l[1], y + l[2]));
    ((sa / n).powi(2) + (sb / n).powi(2)).sqrt()
}

/// Share of pixels with any channel at 255 (blown) or all channels at 0 (crushed).
pub fn clipped(a: &[u8]) -> f64 {
    a.chunks(4).filter(|p| p[..3].iter().any(|&v| v == 255) || p[..3].iter().all(|&v| v == 0)).count() as f64 / (a.len() / 4) as f64
}

/// Peak signal to noise ratio in dB over RGB.
pub fn psnr(a: &[u8], b: &[u8]) -> f64 {
    let mut se = 0f64;
    let mut n = 0usize;
    for (p, q) in a.chunks(4).zip(b.chunks(4)) {
        for c in 0..3 {
            let d = p[c] as f64 - q[c] as f64;
            se += d * d;
            n += 1;
        }
    }
    10.0 * (255.0f64.powi(2) / (se / n as f64).max(1e-9)).log10()
}
