//! Choosing LightCraft develop settings from [`Stats`].
//!
//! Every setting is a function of how far one measurement sits outside a target band and is 0
//! inside it: a photo that measures well gets no change (the do no harm rule). The bands come
//! from the CC0 fixtures (tests/fixtures) and are pinned by tests/params.rs and tests/quality.rs.

use lightcraft_color::transfer::{linear_to_srgb, srgb_to_linear};
use lightcraft_develop::{DevelopSettings, WbMode};
use lightcraft_geom::Point;

use crate::stats::Stats;

/// Median (sRGB encoded luminance) that exposure aims at.
const TARGET_MEDIAN: f32 = 0.45;
/// Brightest tone (99.5th percentile) exposure may lift to.
const HEADROOM_TOP: f32 = 0.97;
/// Exposure changes smaller than this (EV) are dropped.
const EV_DEAD: f32 = 0.3;

/// The chosen corrections, in LightCraft's slider units.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Choice {
    pub exposure: f64,
    pub shadows: f64,
    pub highlights: f64,
    pub contrast: f64,
    /// Levels on the master point curve (encoded input that maps to 0 and to 1).
    pub black: f64,
    pub white: f64,
    pub vibrance: f64,
    /// White balance: 0 keeps the photo's own, 1 neutralises the grey world estimate fully.
    pub wb_strength: f64,
    pub temp: f64,
    pub tint: f64,
    pub nr_luminance: f64,
    pub nr_color: f64,
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn round(v: f32, step: f32) -> f64 {
    ((v / step).round() * step) as f64
}

pub fn choose(s: &Stats) -> Choice {
    let lin = |e: f32| srgb_to_linear(e.clamp(0.0, 1.0)).max(1e-4);

    // Exposure: bring the median to the target, but only as far as the brightest tones allow.
    // What the headroom does not allow is left to Shadows.
    let ev_mid = (lin(TARGET_MEDIAN) / lin(s.p50)).log2();
    // (headroom of the brightest channel, so saturated colours do not clip)
    let headroom = (lin(HEADROOM_TOP) / lin(s.p995_max)).log2();
    let mut deficit = 0f32;
    // darken only when something says it is overexposed: blown pixels or no dark tones at all
    let blown = -1.2 * smoothstep(0.02, 0.15, s.clip_hi);
    let mut ev = if ev_mid > 0.0 {
        let ev = ev_mid.min(headroom + 0.1).max(0.0);
        deficit = ev_mid - ev;
        ev
    } else {
        ev_mid.max(-1.5) * smoothstep(0.01, 0.06, s.clip_hi).max(smoothstep(0.35, 0.5, s.p05))
    };
    if blown < 0.0 && blown < ev {
        (ev, deficit) = (blown, 0.0);
    }
    if ev.abs() < EV_DEAD {
        ev = 0.0;
    }
    let gain = 2f32.powf(ev);
    let shadows = ((deficit - 1.0) * 12.0).clamp(0.0, 40.0);
    let highlights = -(((lin(s.p995_max) * gain / lin(HEADROOM_TOP)) - 1.2) * 200.0).clamp(0.0, 40.0);

    // Levels: stretch a lifted black or a low white (haze, flat light) after exposure.
    let after = |e: f32| linear_to_srgb((lin(e) * gain).min(1.0));
    let (lo, hi) = (after(s.p005_min), after(s.p995_max));
    let mut black = smoothstep(0.10, 0.16, lo) * (lo - 0.05).max(0.0);
    let mut white = 1.0 - smoothstep(0.90, 0.84, hi) * (1.0 - (hi + 0.04).min(1.0));
    if white - black < 0.3 {
        (black, white) = (0.0, 1.0);
    }
    let spread = (after(s.p95) - after(s.p05)) / (white - black);
    let contrast = ((0.6 - spread) * 80.0).clamp(0.0, 20.0);
    let vibrance = ((0.025 - s.chroma) * 800.0).clamp(0.0, 20.0);

    // White balance: partly, by the size of the cast (mean of the grey world and white patch
    // estimates), their agreement and how much one hue dominates a colourful photo (sunsets and
    // foliage are not casts).
    let (g, wp) = (s.grey_ab, s.white_ab);
    let len = |v: [f32; 2]| (v[0] * v[0] + v[1] * v[1]).sqrt();
    let cast = len([(g[0] + wp[0]) / 2.0, (g[1] + wp[1]) / 2.0]);
    let agree = if len(wp) < 0.004 || len(g) < 0.002 { 0.6 } else { smoothstep(-0.2, 0.8, (g[0] * wp[0] + g[1] * wp[1]) / (len(g) * len(wp))) };
    let one_hue = smoothstep(0.75, 1.0, s.dominant_hue) * smoothstep(0.02, 0.05, s.chroma);
    let wb_strength = 0.9 * smoothstep(0.006, 0.025, cast) * agree * (1.0 - 0.7 * one_hue);
    let m0 = 1e6 / 6500.0;
    let temp = 1e6 / (m0 + wb_strength * (1e6 / s.auto_wb[0].max(1000.0) - m0));
    // (grey world tint reads foliage as a green cast, so it is trusted only so far)
    let tint = wb_strength * s.auto_wb[1].clamp(-25.0, 25.0);

    // Noise reduction: the measured noise as it will be after the lift.
    let lifted = s.noise * gain * (1.0 + shadows / 100.0);
    let nr_luminance = ((lifted - 8.0) * 4.0).clamp(0.0, 50.0);
    let nr_color = ((lifted - 8.0) * 3.0).clamp(0.0, 30.0);

    Choice {
        exposure: round(ev, 0.01),
        shadows: round(shadows, 1.0),
        highlights: round(highlights, 1.0),
        contrast: round(contrast, 1.0),
        black: round(black, 0.001),
        white: round(white, 0.001),
        vibrance: round(vibrance, 1.0),
        wb_strength: round(wb_strength, 0.01),
        temp: round(temp, 1.0),
        tint: round(tint, 0.1),
        nr_luminance: round(nr_luminance, 1.0),
        nr_color: round(nr_color, 1.0),
    }
}

impl Choice {
    /// Every correction as LightCraft develop settings.
    pub fn settings(&self) -> DevelopSettings {
        let mut d = DevelopSettings::default();
        d.light.exposure = self.exposure;
        d.light.shadows = self.shadows;
        d.light.highlights = self.highlights;
        d.light.contrast = self.contrast;
        if self.black > 0.0 || self.white < 1.0 {
            d.curve.master = vec![Point { x: self.black, y: 0.0 }, Point { x: self.white, y: 1.0 }];
        }
        d.color.vibrance = self.vibrance;
        if self.wb_strength > 0.0 {
            d.wb.mode = WbMode::Custom;
            d.wb.temp = self.temp;
            d.wb.tint = self.tint;
        }
        d.detail.nr_luminance = self.nr_luminance;
        d.detail.nr_color = self.nr_color;
        d
    }

    /// Only the pointwise corrections: no local tone (shadows, highlights) and no noise reduction.
    pub fn global_settings(&self) -> DevelopSettings {
        crate::render::global_only(&self.settings())
    }
}
