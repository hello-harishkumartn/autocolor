# AutoColor design

* **Author:** Harish Kumar T N
* **Date:** 2026-11-15
* **Status:** Reviewed (see Review notes at the end)

## Overview

AutoColor is a static web page at `https://harishkumartn.in/projects/autocolor/` that colour corrects photos with no sliders. You drop photos (or pick them from Google Drive) and get corrected JPEGs back. All work happens in your browser: a Rust engine compiled to WebAssembly measures each photo and renders it through the develop pipeline of [LightCraft](https://github.com/storytold/lightcraft) (a Rust photo editor by storytold, MIT or Apache-2.0).

## The Problem

Phone and camera photos often come out too dark, too bright, flat or tinted. Fixing them needs an editor and some skill. Harish wants a free page on his site that does a sensible fix in one step, never uploads photos and never makes a good photo worse.

## Goals

1. One step correction of exposure, white balance, shadows, highlights, black and white points, contrast, vibrance and noise.
2. Good photos change only a little (mean deltaE00 under 3 on the CC0 fixture set).
3. JPEG, PNG and WebP input (HEIC where the browser decodes it). JPEG output at the original resolution named `<name>-autocolor.jpg`.
4. Batch input with progress, separate downloads and one zip.
5. Google Drive input with the non-sensitive `drive.file` scope.
6. Runs on phones and desktops from GitHub Pages: no server, no cookies, no analytics. WASM under 2 MB.

## Non-Goals

* RAW files (the LightCraft RAW path is too large for the budget). The page says so.
* Manual sliders or presets.
* Keeping EXIF or ICC metadata. Output is sRGB without metadata (this also strips GPS).
* Picking a whole Drive folder in one click (needs a restricted scope).

## Critical User Journeys

### CUJ 1: Fix one photo
User drops `IMG_1.jpg`. A card shows progress, then a before and after slider. User presses Download and gets `IMG_1-autocolor.jpg` at full size.

### CUJ 2: Fix a batch
User selects 20 photos. They are corrected one after another with "3 of 20" progress. Each card has its own Download button. "Download all as zip" saves `autocolor-photos.zip`.

### CUJ 3: Fix photos from Drive
User presses "Choose from Google Drive", signs in, opens a folder in the Picker, selects the photos and confirms. The page downloads each file with the Drive API, corrects it and adds it to the list. Nothing is written back to Drive.

### CUJ 4: Drive not configured yet
`config.js` has empty IDs. The Drive button is disabled and reads "Google Drive setup pending". Everything else works.

## Technical Plan

```
main thread (app.js)                 worker (worker.js)                    engine.wasm (Rust)
file input / drop / Drive  --File--> createImageBitmap(from-image)
                                     OffscreenCanvas -> RGBA8 --ptr--> ac_correct(ptr, w, h, path)
                                                                         stats -> params -> render
                                     RGBA8 -> canvas -> JPEG blob <--
cards, slider, zip  <--blob+report--
```

### Engine (Rust, `engine/`)

Depends on `lightcraft-pipeline`, `lightcraft-develop`, `lightcraft-raster`, `lightcraft-color` and `lightcraft-geom` (pinned to commit `80de5f8faee3bfe33202d0f8373122645dba2988`). Exports a plain C ABI with no wasm-bindgen.

1. **Measure** (`stats.rs`). From a proxy of at most 512 px on the long edge: luminance percentiles (0.5, 5, 50, 95, 99.5) on sRGB encoded luminance, log average luminance, clipped fractions, mean Oklab chroma, a grey world estimate (LightCraft `auto_wb`), a white patch estimate (brightest 2 percent of unclipped pixels) and the share of the dominant hue. Noise sigma is measured at full resolution on up to 4 tiles of 256 px with Immerkaer's Laplacian method, using the median of the tiles.
2. **Choose** (`params.rs`). Each setting is a function of how far one statistic sits outside a target band. Inside the band the setting is 0 (this is the do no harm rule). Percentiles are of sRGB encoded luminance unless named `_min` or `_max` (smallest or largest channel). `lin()` is the sRGB decode.

   | Setting (LightCraft field) | Statistic | Rule |
   |---|---|---|
   | Exposure `light.exposure` | median `p50`, top `p995_max` | lift `log2(lin(0.45)/lin(p50))` capped by headroom `log2(lin(0.97)/lin(p995_max)) + 0.1`. Darken by the same formula (at most 1.5 EV) only when blown pixels (2 channels at 255) exceed 1 percent or `p05 > 0.35`. At least 1.2 EV darker when 15 percent are blown. Changes under 0.3 EV are dropped |
   | Shadows `light.shadows` | lift the headroom did not allow (EV) | 12 per EV above 1 EV, at most 40 |
   | Highlights `light.highlights` | `p995_max` after exposure | negative only when it would pass 1.2 x the top target, at most -40 |
   | Levels `curve.master` (black, white) | `p005_min`, `p995_max` after exposure | black `p005_min - 0.05` faded in from 0.10 to 0.16. White `p995_max + 0.04` faded in from 0.90 down to 0.84 |
   | Contrast `light.contrast` | 5 to 95 spread after levels | `(0.6 - spread) x 80`, 0 to 20 |
   | Vibrance `color.vibrance` | mean Oklab chroma | `(0.025 - chroma) x 800`, 0 to 20. LightCraft vibrance leaves colours at chroma 0.22 and above alone and cuts the boost near skin hues |
   | White balance `wb` (Custom, temp, tint) | cast = mean of grey world and white patch Oklab (a, b) | strength `0.9 x smoothstep(0.006, 0.025, cast) x agreement x (1 - 0.7 x one hue)`. Temperature blends from 6500 K towards LightCraft `auto_wb` in mired by the strength. Tint is the strength times the `auto_wb` tint limited to 25 |
   | Noise `detail.nr_luminance`, `detail.nr_color` | noise sigma x exposure gain x (1 + shadows/100) | `(n - 8) x 4` up to 50 and `(n - 8) x 3` up to 30 |

3. **Render** (`render.rs`). Input is converted from sRGB8 to linear Rec.2020 (the pipeline's working space). A default render of a JPEG returns it unchanged within 1 level (tested).
   * **Full path** (`path = 0`): one `lightcraft_pipeline::render` at full size.
   * **Banded path** (`path = 1`). (a) Global part: an identity 33x33x33 grid (sRGB encoded) is rendered with float linear output (`OutputDepth::F32Linear`) and the chosen settings with shadows, highlights and all noise reduction at 0. It is looked up per pixel with trilinear interpolation. (b) Local part: a proxy of at most 1024 px is rendered with all settings except noise reduction and with the global settings. The ratio of their luminance (limited to 0.25 to 4) is the gain, applied with bilinear interpolation. (c) Rows are processed in bands of about 1 MP with 16 rows of overlap: luminance noise reduction with `lightcraft_pipeline::local::denoise` on linear rows (scale 1, colour noise reduction off because it measures the whole frame), then the table, then the gain, then a hash dither of half a level and quantising in place.
   * Peak memory: full path about 90 bytes per pixel. Banded path the RGBA8 photo plus a fixed amount for the 1024 px proxy renders and one band (not measured separately; a 12.6 MP photo ran in Chrome on the banded path).
   * Tested: on 3x enlarged fixtures the banded result is within mean deltaE00 2 of the full render.
4. **Report**. After each call the engine keeps a JSON report readable through `ac_report_ptr` and `ac_report_len`: `{"path": "full" | "banded", "choice": {exposure, shadows, highlights, contrast, black, white, vibrance, wb_strength, temp, tint, nr_luminance, nr_color}, "stats": {p50, p995, clip_hi, chroma, noise}}`.

C ABI: `ac_alloc(len) -> ptr` (null when memory is short), `ac_free(ptr, len)`, `ac_correct(ptr, w, h, path) -> i32` (`path` 0 full or 1 banded. Returns 0 ok, -1 bad size or pointer, -2 unknown path), `ac_full_bytes(w, h) -> f64`, `ac_report_ptr()`, `ac_report_len()`. A WASM trap (out of memory or a bug) is caught in JS: the instance is dropped and the photo is retried once on the banded path. The file is `engine.wasm` in the app folder and `engine.wasm.b64` in the site repo. `engine.js` always fetches `engine.wasm`.

### Worker (`app/worker.js`, `app/engine.js`)

* Decodes with `createImageBitmap(blob, { imageOrientation: 'from-image', colorSpaceConversion: 'default', premultiplyAlpha: 'none' })`, so EXIF orientation is applied and colour managed input (Display P3 and other ICC profiles) arrives as sRGB. Transparent pixels are composited onto white before correction.
* Types: JPEG, PNG and WebP by MIME type or extension. HEIC and HEIF (often an empty MIME type, so also by extension) are tried. If the browser cannot decode them the card says "HEIC is not supported by this browser. Open it in Safari or convert it to JPEG first." RAW extensions (dng, cr2, cr3, nef, arw, raf, orf, rw2) get "RAW files are not supported. Export a JPEG from your camera app first."
* Size limits: photos over 50 MP get "This photo is N MP. AutoColor handles up to 50 MP." Each photo's canvas is checked by writing and reading back its last pixel (browsers that cannot hold a canvas that large return blank pixels). If that fails the card says "This photo is N MP, which is more than this browser can handle. Try a desktop browser." Above 40 MP the banded path is forced (the full path would pass 4 GB).
* Path choice: full when the photo is at most 12 MP on desktop or 4 MP on phones (coarse pointer) and a probe allocation of `ac_full_bytes` through `ac_alloc` succeeds (it is freed at once and stays in the allocator's pool). Otherwise banded.
* Encodes with `OffscreenCanvas.convertToBlob({ type: 'image/jpeg', quality: 0.92 })`.
* Returns the output blob, before and after previews (at most 1200 px) and the report. `engine-client.js` on the main thread owns the worker: it restarts it after any photo over 12 MP so WASM memory is returned and replaces it if it crashes.

### Main thread (`app/`)

* `app.js`: drop zone, file input, cards, compare slider (a range input over two stacked images), download links and zip button.
* `queue.js`: runs jobs one at a time and reports progress. Testable without a browser.
* `names.js`: output names `<base>-autocolor.jpg`, unique names inside a zip, supported type checks.
* `zip.js`: store only zip writer (CRC32, UTF-8 flag, DOS date) that returns a Blob built from parts.
* `engine-client.js`: the main thread's handle on the worker (see above).
* `drive-config.js`: scope, Picker MIME types and the pending or ready state, testable without a browser.
* `drive.js` and `config.js`: Google Identity Services token client plus Picker. `config.js` defines `GOOGLE_CLIENT_ID`, `GOOGLE_API_KEY` and `GOOGLE_APP_ID` (the Cloud project number), all empty by default. The Drive button shows "Google Drive setup pending" and is disabled while any is empty.

### Google Drive

* Scope `https://www.googleapis.com/auth/drive.file` only. Picker `DocsView` with image MIME types, folders shown for browsing but not selectable, multi-select on, `setOAuthToken(token)`, `setAppId(GOOGLE_APP_ID)` and `setDeveloperKey(GOOGLE_API_KEY)`.
* Files are fetched one at a time from `https://www.googleapis.com/drive/v3/files/{id}?alt=media` with the access token. The token is kept in memory only. On a 401 or 403 the batch pauses and the card shows a "Reconnect Google Drive" button (a new token request needs a user click or the popup is blocked).

### Legal and attribution (on the page, in a footer section "Privacy and credits")

* Privacy: "Your photos are processed on your device and are never uploaded. If you use Google Drive, AutoColor reads only the files you pick (with the drive.file permission) and stores nothing. AutoColor's use of information received from Google APIs adheres to the Google API Services User Data Policy, including the Limited Use requirements." with links to Google's Terms of Service, Privacy Policy and the User Data Policy. No cookies, no analytics. Output photos carry no EXIF (location data is removed).
* Credits: "Built on LightCraft by storytold (MIT/Apache-2.0)" linking to the repo, with `licenses/` holding LightCraft's `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE` and AutoColor's `LICENSE`. No third party JS libraries.
* Disclaimer: results are automatic and may not suit every photo; keep your originals.

## Alternatives Considered

* **LightCraft `auto_tone` as is.** Rejected by author: a fixed strong recipe (contrast 68, shadows 52, vibrance 30) changes good photos too much.
* **Whole LightCraft web app.** Rejected by author: about 13 MB.
* **One smooth gain map for large photos.** Rejected by author: it cannot carry white balance, contrast or vibrance and halos at edges.
* **wasm-bindgen.** Rejected by author: a handful of numeric exports does not need it.
* **`drive.readonly` for whole folders.** Rejected by author: restricted scope that needs a security assessment.
* **fflate for zip.** Rejected by author: a store only writer is short and JPEGs do not compress.

## Non-Functional Requirements

* **Privacy:** no network requests except loading the page and, when the user chooses Drive, Google sign-in, Picker and file downloads.
* **Performance:** UI never blocks (engine in a worker). About 0.5 s per MP on a laptop for the full path.
* **Accessibility:** labels, visible focus, keyboard operable slider and buttons, contrast AA, Lighthouse accessibility target 100.
* **Responsive:** works at 390x844.
* **Style:** dark theme matching harishkumartn.in (`#0d0b09` background, `#f5efe6` text, `#fb923c` accent), system fonts, inline SVG only. Copy has no Oxford comma, no comma before "and" or "or" and no long dashes.

## Detailed Implementation Plan

### Target files

* `engine/Cargo.toml` (create): crate `autocolor-engine`, cdylib plus rlib, LightCraft git deps at the pinned commit, a local path override documented for builds against `/tmp/lightcraft`, release profile `opt-level = "z"`, LTO, one codegen unit, `panic = "abort"`.
* `engine/src/lib.rs` (create): C ABI and `correct_rgba8`.
* `engine/src/stats.rs`, `engine/src/params.rs`, `engine/src/render.rs` (create): as above, unit tests inline.
* `engine/tests/quality.rs` (create): objective tests on fixtures and synthetic degradations.
* `engine/examples/samples.rs` (create): writes `samples/` before and after images.
* `app/index.html`, `app/styles.css`, `app/app.js`, `app/worker.js`, `app/engine.js`, `app/queue.js`, `app/names.js`, `app/zip.js`, `app/drive.js`, `app/config.js` (create).
* `app/licenses/` (create): LightCraft `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE` and AutoColor `LICENSE`.
* `tests/js/*.test.mjs` (create): node:test for zip, names, queue, engine client, Drive config and pending state and the copy style of all user facing text.
* `tests/browser/run.sh`, `serve.mjs`, `verify.py` (create): agent-browser end to end checks and screenshots, a static server that mounts `dist/` at `/projects/autocolor/` and an output checker.
* `engine/examples/probe.rs`, `probe2.rs`, `stats_all.rs`, `why.rs` (create): tuning aids that print how settings move the metrics.
* `tools/package.json` (create): binaryen for wasm-opt (build only). `THIRD_PARTY/rust-crates.txt` (generated): every crate in the engine's tree with its licence, shipped as `licenses/rust-crates.txt`.
* `scripts/build.sh` (create): builds the wasm, runs wasm-opt, assembles `dist/` and `dist-site/` (wasm as base64).
* `README.md`, `LICENSE`, `THIRD_PARTY/`, `.gitignore`, `docs/GOOGLE_SETUP.md`, `GLOSSARY.md` (create).
* `tests/fixtures/` (7 CC0 photos at 512 px, `LICENSES.md`), `samples/` (before and after JPEGs and a side by side PNG made by `engine/examples/samples.rs`), `docs/GOOGLE_SETUP.md` (step by step Cloud setup).
* Site worktree: `web/src/projects.ts` (AutoColor entry: slug `autocolor`, repo `autocolor`, new optional field `appPage: true` so no page is generated for it), `web/scripts/project-pages.mjs` (index lead copy, which is separate from `projectsSection.intro`), `web/tests/projects.test.mjs` and `web/tests/project-pages.test.mjs` (8 projects, pages without the generator marker are skipped and never overwritten), `site/projects/autocolor/` (the app; its `index.html` must never contain the generator marker), generated `site/projects/index.html` and the home build output. The app is the project page itself, so the card's link `/projects/autocolor/` opens the app directly.

### Verification Plan

* Engine: identity gate (default settings change a JPEG by at most 1 level). For each of 7 kinds of damage on 7 fixtures: no photo gets worse by more than 0.5 mean deltaE00, clipping stays within 2 points of the damaged photo or its original, L* moves closer to the original for dark, overexposed and noisy photos, the mean deltaE00 per kind improves by at least 15 percent and the mean cast per cast kind shrinks. Good photos: mean deltaE00 under 3. Banded path within mean deltaE00 2 of the full path.
* JS: unit tests listed above.
* Browser: upload 1 and 3 photos, single and zip download (zip holds 3 valid JPEGs that differ from the input and have better luminance), Drive pending text, no console errors, screenshots at desktop and 390x844, the site build shows the card and loads the app.
* Lighthouse accessibility score.

### Test seams

`render::render` (the LightCraft adapter), `stats::measure`, `params::choose`, `correct_rgba8` (quality on fixtures) and the C ABI. In JS: `zip.js`, `names.js`, `queue.js` and the Drive config state.

## Review notes

Fresh context reviewers (implementation readiness, codebase details, comprehension) found: no numeric bands (added the table), undefined ABI values and errors (added), missing legal copy (added), LUT render must also zero noise reduction and use float output (done), band noise reduction must run on linear rows with scale 1 and no colour noise reduction (done), site tests hard code 7 projects and compare pages (both test files are now in the plan), missing alpha and colour space handling (added), Drive token renewal needs a click (added).
