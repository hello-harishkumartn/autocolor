//! Measurements of a photo that the settings are chosen from.
//!
//! Tone, colour and cast are measured on a proxy of at most [`PROXY`] px on the long edge. Noise
//! is measured at full resolution, because downscaling averages it away.

use lightcraft_color::perceptual::oklab_from_2020;
use lightcraft_color::transfer::{linear_to_srgb, srgb_to_linear};

/// Long edge of the measurement proxy.
pub const PROXY: usize = 512;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stats {
    /// Percentiles (0.5, 5, 50, 95 and 99.5 %) of sRGB encoded luminance, 0..1.
    pub p005: f32,
    pub p05: f32,
    pub p50: f32,
    pub p95: f32,
    pub p995: f32,
    /// 0.5th percentile of the smallest channel and 99.5th of the largest (sRGB encoded): where
    /// levels would crush or clip a colour, not just a grey.
    pub p005_min: f32,
    pub p995_max: f32,
    /// Mean of log2(Y / 0.18) over the proxy (scene log average in EV from middle grey).
    pub log_mean: f32,
    /// Share of full resolution pixels with at least two channels at 255 (blown) / with every
    /// channel at 0 (crushed).
    pub clip_hi: f32,
    pub clip_lo: f32,
    /// Mean Oklab chroma.
    pub chroma: f32,
    /// Grey world estimate: weighted mean Oklab (a, b) of mid tone, low chroma pixels.
    pub grey_ab: [f32; 2],
    /// White patch estimate: mean Oklab (a, b) of the brightest 2 % unclipped pixels.
    pub white_ab: [f32; 2],
    /// Share of the chroma that falls in the strongest 60 degree hue window (1: one hue).
    pub dominant_hue: f32,
    /// Noise sigma in 8 bit levels of luminance, measured in the flattest areas.
    pub noise: f32,
    /// LightCraft's grey world white balance (temperature K, tint) relative to as shot (6500, 0).
    pub auto_wb: [f32; 2],
}

fn srgb_lut() -> Vec<f32> {
    (0..256).map(|v| srgb_to_linear(v as f32 / 255.0)).collect()
}

/// Box averaged proxy in linear sRGB, at most `max` px on the long edge, plus its size.
pub(crate) fn proxy(rgba: &[u8], w: usize, h: usize, max: usize) -> (Vec<[f32; 3]>, usize, usize) {
    let lut = srgb_lut();
    let k = w.max(h).div_ceil(max).max(1);
    let (pw, ph) = (w.div_ceil(k), h.div_ceil(k));
    let mut acc = vec![[0f32; 4]; pw * ph];
    for y in 0..h {
        let row = &rgba[y * w * 4..(y + 1) * w * 4];
        let pr = &mut acc[(y / k) * pw..(y / k + 1) * pw];
        for (x, p) in row.chunks_exact(4).enumerate() {
            let a = &mut pr[x / k];
            a[0] += lut[p[0] as usize];
            a[1] += lut[p[1] as usize];
            a[2] += lut[p[2] as usize];
            a[3] += 1.0;
        }
    }
    (acc.into_iter().map(|a| [a[0] / a[3], a[1] / a[3], a[2] / a[3]]).collect(), pw, ph)
}

fn luminance(c: [f32; 3]) -> f32 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

fn percentile(sorted: &[f32], q: f32) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    sorted[((sorted.len() - 1) as f32 * q).round() as usize]
}

pub fn measure(rgba: &[u8], w: usize, h: usize) -> Stats {
    let (px, pw, ph) = proxy(rgba, w, h, PROXY);
    let working = crate::render::working_from_linear(px.iter().copied(), pw, ph);
    let mut lum: Vec<f32> = px.iter().map(|&c| linear_to_srgb(luminance(c)).clamp(0.0, 1.0)).collect();
    let log_mean = px.iter().map(|&c| (luminance(c).max(1e-5) / 0.18).log2()).sum::<f32>() / px.len().max(1) as f32;
    let lab: Vec<[f32; 3]> = working.data.iter().map(|&c| oklab_from_2020(c)).collect();

    // grey world: mid tones, weighted towards low chroma
    let (mut ga, mut gb, mut gw) = (0f64, 0f64, 0f64);
    let mut chroma = 0f64;
    let mut hues = [0f64; 36];
    for (l, &e) in lab.iter().zip(&lum) {
        let c = (l[1] * l[1] + l[2] * l[2]).sqrt();
        chroma += c as f64;
        if (0.15..0.92).contains(&e) {
            let wgt = ((1.0 - c / 0.06).max(0.0) as f64).powi(2);
            ga += l[1] as f64 * wgt;
            gb += l[2] as f64 * wgt;
            gw += wgt;
        }
        if c > 0.03 {
            let deg = l[2].atan2(l[1]).to_degrees().rem_euclid(360.0);
            hues[(deg / 10.0) as usize % 36] += c as f64;
        }
    }
    let grey_ab = if gw > 0.0 { [(ga / gw) as f32, (gb / gw) as f32] } else { [0.0; 2] };
    let total: f64 = hues.iter().sum();
    let dominant_hue = if total > 0.0 { (0..36).map(|i| (0..6).map(|j| hues[(i + j) % 36]).sum::<f64>()).fold(0.0, f64::max) / total } else { 0.0 } as f32;

    // white patch: the brightest 2 % of proxy pixels whose channels are all below clipping
    let mut order: Vec<usize> = (0..px.len()).filter(|&i| px[i].iter().all(|&v| v < 0.97)).collect();
    order.sort_by(|&a, &b| lum[b].total_cmp(&lum[a]));
    let top = &order[..(order.len() / 50).max(1).min(order.len())];
    let white_ab = if top.is_empty() {
        [0.0; 2]
    } else {
        let n = top.len() as f32;
        [top.iter().map(|&i| lab[i][1]).sum::<f32>() / n, top.iter().map(|&i| lab[i][2]).sum::<f32>() / n]
    };

    let (mut hi, mut lo) = (0usize, 0usize);
    for p in rgba.chunks_exact(4).take(w * h) {
        hi += ((p[0] == 255) as u8 + (p[1] == 255) as u8 + (p[2] == 255) as u8 >= 2) as usize;
        lo += (p[0] == 0 && p[1] == 0 && p[2] == 0) as usize;
    }
    let n = (w * h).max(1) as f32;
    lum.sort_by(f32::total_cmp);
    let mut mins: Vec<f32> = px.iter().map(|c| linear_to_srgb(c[0].min(c[1]).min(c[2]).clamp(0.0, 1.0))).collect();
    let mut maxs: Vec<f32> = px.iter().map(|c| linear_to_srgb(c[0].max(c[1]).max(c[2]).clamp(0.0, 1.0))).collect();
    mins.sort_by(f32::total_cmp);
    maxs.sort_by(f32::total_cmp);
    Stats {
        p005: percentile(&lum, 0.005),
        p05: percentile(&lum, 0.05),
        p50: percentile(&lum, 0.5),
        p95: percentile(&lum, 0.95),
        p995: percentile(&lum, 0.995),
        p005_min: percentile(&mins, 0.005),
        p995_max: percentile(&maxs, 0.995),
        log_mean,
        clip_hi: hi as f32 / n,
        clip_lo: lo as f32 / n,
        chroma: (chroma / lab.len().max(1) as f64) as f32,
        grey_ab,
        white_ab,
        dominant_hue,
        noise: noise_sigma(rgba, w, h),
        auto_wb: auto_wb(&working),
    }
}

/// Immerkaer's fast noise estimate on 8 bit luma, per 16 x 16 block of up to five 256 px tiles at
/// full resolution; the 10th percentile of the blocks (the flattest areas) is the photo's noise.
pub fn noise_sigma(rgba: &[u8], w: usize, h: usize) -> f32 {
    const TILE: usize = 256;
    const B: usize = 16;
    if w < 3 || h < 3 {
        return 0.0;
    }
    let luma = |x: usize, y: usize| {
        let p = &rgba[(y * w + x) * 4..];
        0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32
    };
    let (tw, th) = (TILE.min(w), TILE.min(h));
    let centres = [(w / 2, h / 2), (w / 4, h / 4), (3 * w / 4, h / 4), (w / 4, 3 * h / 4), (3 * w / 4, 3 * h / 4)];
    let mut blocks = Vec::new();
    for (cx, cy) in centres {
        let x0 = cx.saturating_sub(tw / 2).min(w - tw);
        let y0 = cy.saturating_sub(th / 2).min(h - th);
        let mut by = y0 + 1;
        while by + B < y0 + th {
            let mut bx = x0 + 1;
            while bx + B < x0 + tw {
                let mut sum = 0f32;
                for y in by..by + B {
                    for x in bx..bx + B {
                        let v = luma(x - 1, y - 1) - 2.0 * luma(x, y - 1) + luma(x + 1, y - 1) - 2.0 * luma(x - 1, y) + 4.0 * luma(x, y)
                            - 2.0 * luma(x + 1, y)
                            + luma(x - 1, y + 1)
                            - 2.0 * luma(x, y + 1)
                            + luma(x + 1, y + 1);
                        sum += v.abs();
                    }
                }
                blocks.push(sum * (std::f32::consts::PI / 2.0).sqrt() / (6.0 * (B * B) as f32));
                bx += B;
            }
            by += B;
        }
    }
    blocks.sort_by(f32::total_cmp);
    percentile(&blocks, 0.1)
}

/// LightCraft's `auto_wb` on the proxy (linear Rec.2020).
fn auto_wb(working: &lightcraft_raster::Rgb32f) -> [f32; 2] {
    let (t, tint) = lightcraft_pipeline::auto::auto_wb(working, &lightcraft_pipeline::SourceInfo::default());
    [t as f32, tint as f32]
}
