# AutoColor anchored hypothesis (grill outcome)

AutoColor is a static web page at harishkumartn.in/projects/autocolor/ that colour corrects photos with no sliders. A Rust engine compiled to WASM (no wasm-bindgen, plain C ABI, about 1 MB) measures each photo on a small proxy and picks LightCraft develop settings (white balance, exposure, shadows, highlights, whites, blacks, contrast, vibrance, noise reduction), then renders through LightCraft's own CPU pipeline (lightcraft-pipeline, -develop, -color, -raster at commit 80de5f8). Photos never leave the device; Google Drive input uses the drive.file scope with the Picker.

## Decisions taken while grilling (answered from code and docs, no human available)
1. Engine reuse: depend on lightcraft-pipeline. Spike: release wasm with render + auto_tone is 961 KB, no imports, 800x600 renders in 0.12 s in Node. LightCraft's auto_tone is a fixed Lightroom-like recipe (contrast 68, shadows 52, vibrance 30) which changes good photos too much, so AutoColor writes its own parameter selection and reuses lightcraft's auto_wb (grey world) and scene_log_mean ideas.
2. Memory: the pipeline peaks near 82 bytes per pixel (12 MP used 985 MB). Full pipeline render only when the photo is below a pixel budget (about 12 MP desktop, 4 MP phone). Above it the engine renders a proxy at the budget through the pipeline (no noise reduction) and transfers the result to full resolution as a smooth per-pixel gain map in linear light. If WASM runs out of memory the worker restarts and retries on the gain-map path with a smaller proxy. Hard cap 50 MP (and the browser's canvas limit) with a clear message.
3. Decode and encode in the browser: createImageBitmap with imageOrientation from-image (EXIF orientation), OffscreenCanvas getImageData in a Web Worker, convertToBlob image/jpeg quality 0.92. HEIC works only where the browser decodes it (Safari); elsewhere a clear message. RAW not supported (message).
4. Google Drive: drive.file scope only (non-sensitive). Picking a folder with drive.file does not grant access to its children (Google docs are silent; developer reports say no), so the Picker shows folders for navigation and users multi-select the image files inside. No restricted scope. setAppId with the project number is required so picked files are readable.
5. Zip: minimal store-only zip writer in JS (JPEGs do not compress), own code, CRC32 table.
6. No wasm-bindgen: exports alloc/free/analyse/correct; JS glue is hand written and small.

## Goldfish brainstorm findings (zoom-out, design, inventive catalyst) and how they change the plan
- A single smooth gain map cannot carry white balance, contrast or vibrance and would halo. Changed: above the memory budget the engine renders a 3D lookup table of the global settings through the LightCraft pipeline (an identity LUT image rendered with local tone switched off) and applies it per pixel at full resolution. Only the local shadows and highlights part travels as a low resolution luminance gain (full render divided by global-only render on the proxy).
- Noise reduction must not vanish on big noisy phone photos. Changed: on the large path noise reduction runs at full resolution in overlapping row bands through the LightCraft pipeline (noise only settings, native_long set to the photo's long edge).
- Noise must be measured at full resolution, not on the proxy. Changed: noise is estimated on full resolution sample tiles.
- The base render of a JPEG with default settings must be close to identity, else every photo changes. Changed: first engine test is that gate (mean deltaE00 under 1).
- Do no harm needs a rule. Changed: every setting is a function of how far a measured statistic sits outside a target band (dead zone inside the band). White balance is applied only partly, scaled by cast strength and by agreement between grey world and white patch estimates.
- Memory: probe with memory.grow before the full render instead of waiting for a crash; restart the worker after big photos. The iOS canvas limit (about 16.7 MP) is probed and reported as a clear message.
- 8 bit posterisation after lifts: the large path dithers before quantising.
- Zip: store only, unique names, UTF-8 flag, DOS dates, built as a Blob from parts.
- Drive: Picker filtered to supported image types, folders open but not selectable, one download at a time, token re-requested when it expires.
- EXIF and ICC are dropped by canvas encoding: documented (also removes GPS which helps privacy).

## Alternatives weighed
- LightCraft auto_tone as is: rejected, it applies a fixed strong recipe (contrast 68, shadows 52, vibrance 30) to every photo which breaks do no harm.
- Whole lightcraft-web app: rejected, about 13 MB.
- wasm-bindgen: rejected, the engine needs only a few numeric exports; plain C ABI avoids the tool and version pinning.
- drive.readonly for folder picking: rejected, restricted scope with security assessment; multi-select inside a folder works with drive.file.
- fflate: rejected for size and dependency count; a store only writer is under 100 lines and JPEGs do not compress.
- Decoding in WASM (LightCraft codecs): rejected for size; the browser decoders are already there and apply EXIF orientation.
