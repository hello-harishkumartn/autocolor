//! AutoColor engine: measures a photo, picks LightCraft develop settings and renders them.
//! Built on LightCraft by storytold (MIT OR Apache-2.0), https://github.com/storytold/lightcraft

pub mod abi;
pub mod params;
pub mod render;
pub mod stats;

use render::Path;

/// What the engine measured and chose for the last photo.
#[derive(Clone, Copy, Debug)]
pub struct Report {
    pub stats: stats::Stats,
    pub choice: params::Choice,
    pub path: Path,
}

/// Measures `rgba` (RGBA8, `w` x `h`), chooses corrections and renders them in place.
pub fn correct_rgba8(rgba: &mut [u8], w: usize, h: usize, path: Path) -> Report {
    let stats = stats::measure(rgba, w, h);
    let choice = params::choose(&stats);
    render::render(rgba, w, h, &choice.settings(), path);
    Report { stats, choice, path }
}

impl Report {
    /// The report as a small JSON object (field names as in [`stats::Stats`] and [`params::Choice`]).
    pub fn to_json(&self) -> String {
        let (s, c) = (&self.stats, &self.choice);
        let path = match self.path {
            Path::Full => "full",
            Path::Banded => "banded",
        };
        format!(
            concat!(
                "{{\"path\":\"{}\",\"choice\":{{\"exposure\":{},\"shadows\":{},\"highlights\":{},\"contrast\":{},\"black\":{},\"white\":{},",
                "\"vibrance\":{},\"wb_strength\":{},\"temp\":{},\"tint\":{},\"nr_luminance\":{},\"nr_color\":{}}},",
                "\"stats\":{{\"p50\":{:.3},\"p995\":{:.3},\"clip_hi\":{:.4},\"chroma\":{:.4},\"noise\":{:.2}}}}}"
            ),
            path, c.exposure, c.shadows, c.highlights, c.contrast, c.black, c.white, c.vibrance, c.wb_strength, c.temp, c.tint, c.nr_luminance, c.nr_color,
            s.p50, s.p995, s.clip_hi, s.chroma, s.noise
        )
    }
}
