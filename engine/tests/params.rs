//! Choosing settings from measurements: each setting moves only when its statistic is out of band.
use autocolor_engine::params::choose;
use autocolor_engine::stats::Stats;

/// Measurements of a well exposed, neutral photo (the city fixture).
fn good() -> Stats {
    Stats {
        p005: 0.08, p05: 0.18, p50: 0.44, p95: 0.91, p995: 0.97, p005_min: 0.06, p995_max: 0.99, log_mean: -0.19, clip_hi: 0.001, clip_lo: 0.0,
        chroma: 0.03, grey_ab: [0.001, 0.002], white_ab: [0.0, -0.001], dominant_hue: 0.6, noise: 0.4, auto_wb: [6450.0, 1.0],
    }
}

#[test]
fn a_good_photo_gets_no_tone_change() {
    let c = choose(&good());
    assert_eq!((c.exposure, c.shadows, c.highlights, c.contrast), (0.0, 0.0, 0.0, 0.0), "{c:?}");
    assert_eq!((c.black, c.white), (0.0, 1.0), "{c:?}");
    assert_eq!((c.nr_luminance, c.nr_color), (0.0, 0.0), "{c:?}");
    assert!(c.wb_strength < 0.05, "{c:?}");
}

#[test]
fn a_dark_photo_with_headroom_gets_exposure() {
    // the city fixture darkened by 2 EV
    let s = Stats { p005: 0.02, p05: 0.05, p50: 0.224, p95: 0.48, p995: 0.522, p995_max: 0.55, log_mean: -2.2, clip_hi: 0.0, ..good() };
    let c = choose(&s);
    assert!((1.6..2.3).contains(&c.exposure), "{c:?}");
    assert_eq!(c.highlights, 0.0);
}

#[test]
fn a_dark_scene_with_bright_highlights_gets_only_a_little_lift() {
    // the food fixture: dark background, bright plates
    let s = Stats { p005: 0.012, p05: 0.027, p50: 0.19, p95: 0.88, p995: 0.951, p005_min: 0.01, p995_max: 0.98, log_mean: -2.27, ..good() };
    let c = choose(&s);
    assert!(c.exposure <= 0.3, "{c:?}");
    assert!(c.shadows <= 25.0, "{c:?}");
}

#[test]
fn an_overexposed_photo_is_darkened() {
    let s = Stats { p005: 0.12, p05: 0.25, p50: 0.62, p95: 1.0, p995: 1.0, p995_max: 1.0, log_mean: 0.8, clip_hi: 0.15, ..good() };
    let c = choose(&s);
    assert!(c.exposure < -0.4, "{c:?}");
}

#[test]
fn a_bright_high_key_photo_is_left_alone() {
    // the fog fixture: snow, bright on purpose, nothing clipped
    let s = Stats { p005: 0.039, p05: 0.211, p50: 0.749, p95: 0.973, p995: 0.988, p995_max: 0.99, log_mean: 0.98, clip_hi: 0.008, ..good() };
    assert_eq!(choose(&s).exposure, 0.0);
}

#[test]
fn a_flat_photo_gets_its_black_and_white_points_stretched() {
    let s = Stats { p005: 0.25, p05: 0.31, p50: 0.47, p95: 0.75, p995: 0.79, p005_min: 0.23, p995_max: 0.8, ..good() };
    let c = choose(&s);
    assert!(c.black > 0.12 && c.white < 0.88, "{c:?}");
    assert!(c.black < s.p005 as f64 && c.white > s.p995 as f64, "never clips the measured range: {c:?}");
}

#[test]
fn white_balance_strength_follows_cast_size_and_agreement() {
    let strong = Stats { grey_ab: [0.012, 0.035], white_ab: [0.01, 0.03], auto_wb: [4800.0, -8.0], ..good() };
    let disagree = Stats { grey_ab: [0.012, 0.035], white_ab: [-0.01, -0.02], ..strong };
    let one_hue = Stats { dominant_hue: 0.95, chroma: 0.06, ..strong };
    let (s, d, o) = (choose(&strong), choose(&disagree), choose(&one_hue));
    assert!(s.wb_strength > 0.6, "{s:?}");
    assert!(d.wb_strength < s.wb_strength * 0.5, "{d:?}");
    assert!(o.wb_strength < s.wb_strength * 0.6, "{o:?}");
    // partial: temperature moves from 6500 towards the estimate, never past it
    assert!(s.temp < 6500.0 && s.temp > 4800.0, "{s:?}");
}

#[test]
fn noise_reduction_follows_noise_times_the_lift() {
    let noisy_dark = Stats { p005: 0.01, p05: 0.03, p50: 0.15, p95: 0.4, p995: 0.45, p995_max: 0.5, noise: 3.0, ..good() };
    let c = choose(&noisy_dark);
    assert!(c.nr_luminance > 10.0, "{c:?}");
    let textured_good = Stats { noise: 6.8, ..good() };
    assert_eq!(choose(&textured_good).nr_luminance, 0.0);
}

#[test]
fn settings_are_written_into_lightcraft_develop_settings() {
    let s = Stats { p005: 0.25, p05: 0.31, p50: 0.2, p95: 0.45, p995: 0.5, p005_min: 0.24, p995_max: 0.55, grey_ab: [0.012, 0.035], white_ab: [0.01, 0.03], auto_wb: [4800.0, -8.0], ..good() };
    let c = choose(&s);
    let d = c.settings();
    assert_eq!(d.light.exposure, c.exposure);
    assert_eq!(d.wb.mode, lightcraft_develop::WbMode::Custom);
    assert_eq!(d.curve.master.len(), 2);
    assert_eq!(d.color.vibrance, c.vibrance);
    let g = c.global_settings();
    assert_eq!((g.light.shadows, g.light.highlights, g.detail.nr_luminance, g.detail.nr_color), (0.0, 0.0, 0.0, 0.0));
    assert_eq!(g.light.exposure, c.exposure);
}

#[test]
fn saturated_colours_limit_the_lift() {
    // dark, but the brightest colour channel is already near the top: lifting would clip colours
    let s = Stats { p005: 0.02, p05: 0.05, p50: 0.224, p95: 0.48, p995: 0.52, p995_max: 0.9, ..good() };
    assert!(choose(&s).exposure < 0.6, "{:?}", choose(&s));
}

#[test]
fn heavy_clipping_darkens_even_with_a_dark_median() {
    // overexposed food: the background keeps the median low but many pixels are blown
    let s = Stats { p005: 0.03, p05: 0.06, p50: 0.42, p95: 1.0, p995: 1.0, p995_max: 1.0, clip_hi: 0.17, ..good() };
    assert!(choose(&s).exposure <= -0.5, "{:?}", choose(&s));
}
