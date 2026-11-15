//! Rendering RGBA8 photos through the LightCraft pipeline.
//!
//! The pipeline works in linear Rec.2020 and returns display encoded sRGB. [`Path::Full`] renders
//! the whole photo at once (about 90 bytes per pixel at peak). [`Path::Banded`] keeps the photo as
//! RGBA8 and renders only small images through the pipeline (see [`render_banded`]).

use lightcraft_color::transfer::{linear_to_srgb, srgb_to_linear};
use lightcraft_color::{REC2020, SRGB};
use lightcraft_develop::DevelopSettings;
use lightcraft_pipeline::{DeepSamples, OutputDepth, RenderRequest, SourceInfo};
use lightcraft_raster::{Rgb32f, Rgba8};

/// How a photo is rendered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Path {
    /// One pipeline render at full size.
    Full,
    /// Global settings as a lookup table, local tone as a low resolution gain, noise reduction in
    /// row bands. For photos too large for [`Path::Full`].
    Banded,
}

/// Linear sRGB pixels to linear Rec.2020, the pipeline's working space.
pub(crate) fn working_from_linear(px: impl Iterator<Item = [f32; 3]>, w: usize, h: usize) -> Rgb32f {
    let m = SRGB.to_space(&REC2020).to_f32();
    let data = px.map(|l| [0, 1, 2].map(|r| m[r][0] * l[0] + m[r][1] * l[1] + m[r][2] * l[2])).collect();
    Rgb32f { width: w, height: h, data }
}

/// sRGB8 to linear Rec.2020.
pub(crate) fn to_working(rgba: &[u8], w: usize, h: usize) -> Rgb32f {
    let lut: Vec<f32> = (0..256).map(|v| srgb_to_linear(v as f32 / 255.0)).collect();
    working_from_linear(rgba.chunks_exact(4).take(w * h).map(|p| [lut[p[0] as usize], lut[p[1] as usize], lut[p[2] as usize]]), w, h)
}

/// One pipeline render of `src` at its own size.
pub(crate) fn pipeline(src: &Rgb32f, s: &DevelopSettings) -> Rgba8 {
    let req = RenderRequest::exact(src.width, src.height);
    lightcraft_pipeline::render(src, &SourceInfo::default(), s, &req).image
}

/// Renders `rgba` (RGBA8, `w` x `h`) with `s` in place. Alpha is left as it is.
pub fn render(rgba: &mut [u8], w: usize, h: usize, s: &DevelopSettings, path: Path) {
    match path {
        Path::Full => {
            let out = pipeline(&to_working(rgba, w, h), s);
            for (p, o) in rgba.chunks_exact_mut(4).zip(&out.data) {
                p[..3].copy_from_slice(&o[..3]);
            }
        }
        Path::Banded => render_banded(rgba, w, h, s),
    }
}

/// Only the pointwise settings of `s`: no local tone (shadows, highlights) and no noise reduction.
pub fn global_only(s: &DevelopSettings) -> DevelopSettings {
    let mut g = s.clone();
    g.light.shadows = 0.0;
    g.light.highlights = 0.0;
    g.detail.nr_luminance = 0.0;
    g.detail.nr_color = 0.0;
    g
}

/// Grid points per axis of the lookup table.
const LUT_N: usize = 33;
/// Long edge of the proxy the local tone gain is measured on.
pub const GAIN_PROXY: usize = 1024;
/// Pixels per noise reduction band, and rows of overlap above and below it.
const BAND_PIXELS: usize = 1 << 20;
const OVERLAP: usize = 16;

/// A pipeline render returned as linear sRGB floats.
fn pipeline_linear(src: &Rgb32f, s: &DevelopSettings) -> Vec<[f32; 3]> {
    let req = RenderRequest { depth: OutputDepth::F32Linear, ..RenderRequest::exact(src.width, src.height) };
    let r = lightcraft_pipeline::render(src, &SourceInfo::default(), s, &req);
    match r.deep.map(|d| d.samples) {
        Some(DeepSamples::F32(v)) => v.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect(),
        // (not reached: F32Linear always returns float samples) fall back to the 8-bit image
        _ => r.image.data.iter().map(|p| [0, 1, 2].map(|i| srgb_to_linear(p[i] as f32 / 255.0))).collect(),
    }
}

fn luma(c: [f32; 3]) -> f32 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

/// The pointwise settings as a table: sRGB encoded input grid to sRGB encoded output.
struct Lut(Vec<[f32; 3]>);

impl Lut {
    fn new(s: &DevelopSettings) -> Lut {
        let n = LUT_N;
        let mut grid = Vec::with_capacity(n * n * n * 4);
        for b in 0..n {
            for g in 0..n {
                for r in 0..n {
                    let e = |i: usize| (i as f32 * 255.0 / (n - 1) as f32).round() as u8;
                    grid.extend_from_slice(&[e(r), e(g), e(b), 255]);
                }
            }
        }
        let out = pipeline_linear(&to_working(&grid, n * n, n), s);
        Lut(out.into_iter().map(|c| c.map(|v| linear_to_srgb(v.clamp(0.0, 1.0)))).collect())
    }

    /// Trilinear lookup of an sRGB encoded colour (0..1).
    fn get(&self, c: [f32; 3]) -> [f32; 3] {
        let n = LUT_N;
        let f = c.map(|v| v.clamp(0.0, 1.0) * (n - 1) as f32);
        let i = f.map(|v| (v as usize).min(n - 2));
        let t = [f[0] - i[0] as f32, f[1] - i[1] as f32, f[2] - i[2] as f32];
        let at = |r: usize, g: usize, b: usize| self.0[(b * n + g) * n + r];
        let mut out = [0f32; 3];
        for (dr, wr) in [(0, 1.0 - t[0]), (1, t[0])] {
            for (dg, wg) in [(0, 1.0 - t[1]), (1, t[1])] {
                for (db, wb) in [(0, 1.0 - t[2]), (1, t[2])] {
                    let w = wr * wg * wb;
                    let p = at(i[0] + dr, i[1] + dg, i[2] + db);
                    for k in 0..3 {
                        out[k] += w * p[k];
                    }
                }
            }
        }
        out
    }
}

/// Local tone (shadows, highlights) as a luminance gain on a proxy: the full render over the
/// render without local tone. `None` when the settings have no local tone.
struct Gain {
    w: usize,
    h: usize,
    g: Vec<f32>,
}

impl Gain {
    fn new(rgba: &[u8], w: usize, h: usize, s: &DevelopSettings, global: &DevelopSettings) -> Option<Gain> {
        if s.light.shadows == 0.0 && s.light.highlights == 0.0 {
            return None;
        }
        let (px, pw, ph) = crate::stats::proxy(rgba, w, h, GAIN_PROXY);
        let src = working_from_linear(px.into_iter(), pw, ph);
        let mut local = s.clone();
        local.detail.nr_luminance = 0.0;
        local.detail.nr_color = 0.0;
        let (a, b) = (pipeline_linear(&src, &local), pipeline_linear(&src, global));
        let g = a.iter().zip(&b).map(|(a, b)| ((luma(*a) + 1e-4) / (luma(*b) + 1e-4)).clamp(0.25, 4.0)).collect();
        Some(Gain { w: pw, h: ph, g })
    }

    /// Bilinear gain at full resolution pixel (`x`, `y`) of a `w` x `h` photo.
    fn at(&self, x: usize, y: usize, w: usize, h: usize) -> f32 {
        let fx = ((x as f32 + 0.5) * self.w as f32 / w as f32 - 0.5).clamp(0.0, (self.w - 1) as f32);
        let fy = ((y as f32 + 0.5) * self.h as f32 / h as f32 - 0.5).clamp(0.0, (self.h - 1) as f32);
        let (x0, y0) = (fx as usize, fy as usize);
        let (x1, y1) = ((x0 + 1).min(self.w - 1), (y0 + 1).min(self.h - 1));
        let (tx, ty) = (fx - x0 as f32, fy - y0 as f32);
        let g = |x: usize, y: usize| self.g[y * self.w + x];
        (g(x0, y0) * (1.0 - tx) + g(x1, y0) * tx) * (1.0 - ty) + (g(x0, y1) * (1.0 - tx) + g(x1, y1) * tx) * ty
    }
}

/// A small per pixel offset in -0.5..0.5 (8-bit steps) so smooth gradients do not band.
fn dither(x: usize, y: usize) -> f32 {
    let mut v = (x as u32).wrapping_mul(0x9E37_79B1) ^ (y as u32).wrapping_mul(0x85EB_CA77);
    v ^= v >> 15;
    v = v.wrapping_mul(0x2C1B_3C6D);
    v ^= v >> 12;
    (v & 0xFFFF) as f32 / 65536.0 - 0.5
}

/// The banded path: memory stays near 4 bytes per pixel of the photo plus a fixed amount.
fn render_banded(rgba: &mut [u8], w: usize, h: usize, s: &DevelopSettings) {
    let global = global_only(s);
    let lut = Lut::new(&global);
    let gain = Gain::new(rgba, w, h, s, &global);
    let mut nr = DevelopSettings::default();
    nr.detail = s.detail;
    // colour noise reduction measures the whole frame; it is not run in bands
    nr.detail.nr_color = 0.0;
    let denoise = s.detail.nr_luminance > 0.0;
    let to_srgb = REC2020.to_space(&SRGB).to_f32();
    let dec: Vec<f32> = (0..256).map(|v| v as f32 / 255.0).collect();

    let rows = (BAND_PIXELS / w.max(1)).clamp(32, h.max(32));
    let mut carry: Vec<u8> = Vec::new(); // original rows above the current band (its top overlap)
    let mut y0 = 0;
    while y0 < h {
        let y1 = (y0 + rows).min(h);
        let below = (y1 + OVERLAP).min(h);
        // the band's input colours, sRGB encoded 0..1, rows y0..y1
        let band: Vec<[f32; 3]> = if denoise {
            let above = carry.len() / (w * 4);
            let mut src = carry.clone();
            src.extend_from_slice(&rgba[y0 * w * 4..below * w * 4]);
            let mut img = to_working(&src, w, above + below - y0);
            lightcraft_pipeline::local::denoise(&mut img, &nr, 1.0, None);
            img.data[above * w..(above + y1 - y0) * w]
                .iter()
                .map(|c| [0, 1, 2].map(|r| linear_to_srgb((to_srgb[r][0] * c[0] + to_srgb[r][1] * c[1] + to_srgb[r][2] * c[2]).clamp(0.0, 1.0))))
                .collect()
        } else {
            rgba[y0 * w * 4..y1 * w * 4].chunks_exact(4).map(|p| [dec[p[0] as usize], dec[p[1] as usize], dec[p[2] as usize]]).collect()
        };
        let keep = OVERLAP.min(y1 - y0);
        carry = rgba[(y1 - keep) * w * 4..y1 * w * 4].to_vec();
        for (i, c) in band.into_iter().enumerate() {
            let (x, y) = (i % w, y0 + i / w);
            let mut out = lut.get(c);
            if let Some(g) = &gain {
                let k = g.at(x, y, w, h);
                out = out.map(|v| linear_to_srgb((srgb_to_linear(v) * k).clamp(0.0, 1.0)));
            }
            let d = dither(x, y);
            let p = &mut rgba[(y * w + x) * 4..(y * w + x) * 4 + 3];
            for k in 0..3 {
                p[k] = (out[k] * 255.0 + d).round().clamp(0.0, 255.0) as u8;
            }
        }
        y0 = y1;
    }
}
