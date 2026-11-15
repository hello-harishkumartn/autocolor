# AutoColor

Automatic colour correction for photos, in the browser. Add photos and get corrected copies back with no sliders. Photos never leave your device.

Live at https://harishkumartn.in/projects/autocolor/

## What it does

AutoColor measures each photo (brightness percentiles, highlight headroom, clipped pixels, colour cast from grey world and white patch estimates, colourfulness and noise) and only changes what measures out of range. From those measurements it sets exposure, white balance, shadows, highlights, black and white points, contrast, vibrance and noise reduction, then renders the photo in linear light with the develop pipeline of [LightCraft](https://github.com/storytold/lightcraft) compiled to WebAssembly. Photos that already look balanced are changed only a little.

* Input: JPEG, PNG and WebP (drag and drop or file picker), HEIC where the browser can decode it (Safari), Google Drive files through the Google Picker. RAW is not supported.
* Added photos wait as ready cards until you press Start correcting. Each card and the whole run show a progress bar.
* Output: high quality JPEG at the original size named `<name>-autocolor.jpg`, one by one or as one zip. EXIF orientation is applied. Metadata is not copied.
* Large photos: up to 12 MP (4 MP on phones) are rendered in one pass. Larger photos use a banded path that keeps memory near 4 bytes per pixel. The limit is 50 MP or what the browser's canvas allows.

## How it works

```
app/        static page: index.html, app.js (UI), worker.js (decode, correct, encode in a Web Worker),
            engine.js (WASM glue), zip.js (store only zip), queue.js, names.js, drive.js, config.js
engine/     Rust crate compiled to engine.wasm (plain C ABI, no wasm-bindgen)
  stats.rs    measurements on a 512 px proxy plus noise at full resolution
  params.rs   target bands and the settings chosen from them
  render.rs   LightCraft pipeline adapter: full path and banded path
tests/      Rust quality tests (in engine/tests), JS unit tests, browser checks, CC0 fixtures
docs/       SPEC.md (design), GOOGLE_SETUP.md, DRIFT_REPORT.md
```

The design is in [docs/SPEC.md](docs/SPEC.md).

## Build

Needs Rust (stable, with the `wasm32-unknown-unknown` target) and Node 22 or newer.

```sh
scripts/build.sh          # engine.wasm (wasm-opt -Oz) into dist/, and dist-site/ with engine.wasm.b64
```

The engine depends on LightCraft crates pinned to a commit on GitHub. To build against a local clone instead, copy `engine/.cargo/config.toml.example` to `engine/.cargo/config.toml`.

## Test

The CC0 test photos are stored as base64 text (`tests/fixtures/*.jpg.b64`). Decode them once before running the Rust tests:

```sh
for f in tests/fixtures/*.jpg.b64; do base64 -d "$f" > "${f%.b64}"; done
```

Then run:

```sh
(cd engine && cargo test --release)   # measurements, settings, quality on fixtures, banded path, C ABI
node --test tests/js/*.test.mjs      # zip, names, queue, Drive config, copy style
tests/browser/run.sh                 # agent-browser end to end checks (after scripts/build.sh)
```

## Google Drive

Drive input needs a free Google Cloud project with public IDs in `app/config.js`. Step by step: [docs/GOOGLE_SETUP.md](docs/GOOGLE_SETUP.md). Until then the button shows "Google Drive setup pending".

## Credits and licences

* AutoColor: MIT, see [LICENSE](LICENSE).
* Built on [LightCraft](https://github.com/storytold/lightcraft) by storytold (MIT/Apache-2.0). Licence texts and NOTICE in [THIRD_PARTY/lightcraft](THIRD_PARTY/lightcraft) and shipped in the app's `licenses/` folder.
* Test photos: CC0 from Wikimedia Commons, see [tests/fixtures/LICENSES.md](tests/fixtures/LICENSES.md).
* No third party JavaScript. Build tool: binaryen wasm-opt (Apache-2.0, not shipped).
