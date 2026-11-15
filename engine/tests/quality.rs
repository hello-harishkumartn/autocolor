//! Objective quality on the CC0 fixtures and synthetic degradations of them.
mod common;
mod degrade;
use autocolor_engine::correct_rgba8;
use autocolor_engine::render::Path;
use common::*;
use degrade::*;

fn corrected(src: &[u8], w: usize, h: usize, path: Path) -> Vec<u8> {
    let mut out = src.to_vec();
    correct_rgba8(&mut out, w, h, path);
    out
}

#[test]
fn good_photos_are_barely_changed() {
    let mut worst = (0.0, "");
    for name in FIXTURES {
        let img = fixture(name);
        let out = corrected(&img.rgba, img.w, img.h, Path::Full);
        let de = mean_de(&img.rgba, &out);
        println!("{name:>9}: dE00 {de:.2}");
        if de > worst.0 {
            worst = (de, name);
        }
    }
    assert!(worst.0 < 3.0, "{} changed by mean dE00 {:.2}", worst.1, worst.0);
}

#[test]
fn degraded_photos_move_closer_to_the_original() {
    let mut failures = Vec::new();
    for kind in KINDS {
        let (mut sum_before, mut sum_after, mut cast_before, mut cast_after) = (0.0, 0.0, 0.0, 0.0);
        for name in FIXTURES {
            let img = fixture(name);
            let bad = degrade(&img.rgba, kind);
            let out = corrected(&bad, img.w, img.h, Path::Full);
            let (b, a) = (mean_de(&img.rgba, &bad), mean_de(&img.rgba, &out));
            let (pb, pa) = (psnr(&img.rgba, &bad), psnr(&img.rgba, &out));
            let (lb, la, lo) = (mean_l(&bad), mean_l(&out), mean_l(&img.rgba));
            let (cb, ca) = (cast(&bad), cast(&out));
            let (kb, ka) = (clipped(&bad), clipped(&out));
            println!("{kind:>6} {name:>9}: dE {b:5.2} -> {a:5.2}  PSNR {pb:5.1} -> {pa:5.1}  L {lb:4.1} -> {la:4.1} (orig {lo:4.1})  cast {cb:4.1} -> {ca:4.1}  clip {kb:.3} -> {ka:.3}");
            sum_before += b;
            sum_after += a;
            cast_before += cb;
            cast_after += ca;
            // no photo is made worse, and clipping stays near what the photo (or its original) had
            if a > b + 0.5 {
                failures.push(format!("{kind} {name}: dE {b:.2} -> {a:.2}"));
            }
            let ko = clipped(&img.rgba);
            if ka > kb.max(ko) + 0.02 {
                failures.push(format!("{kind} {name}: clipping {kb:.3} (original {ko:.3}) -> {ka:.3}"));
            }
            match kind {
                "dark" | "over" | "noisy" => {
                    if (la - lo).abs() >= (lb - lo).abs() {
                        failures.push(format!("{kind} {name}: L* {lb:.1} -> {la:.1}, original {lo:.1}"));
                    }
                }
                _ => {}
            }
        }
        println!("{kind:>6} mean dE {:.2} -> {:.2}, mean cast {:.2} -> {:.2}", sum_before / 7.0, sum_after / 7.0, cast_before / 7.0, cast_after / 7.0);
        // every kind of damage is measurably repaired on average
        if sum_after > 0.85 * sum_before {
            failures.push(format!("{kind}: mean dE {:.2} -> {:.2}", sum_before / 7.0, sum_after / 7.0));
        }
        if ["warm", "cool", "green"].contains(&kind) && cast_after >= cast_before {
            failures.push(format!("{kind}: mean cast {:.2} -> {:.2}", cast_before / 7.0, cast_after / 7.0));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
