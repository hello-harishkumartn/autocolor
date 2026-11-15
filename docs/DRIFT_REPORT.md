# Drift report (2026-11-15)

Design documents checked against the code in this repo and the site worktree: `docs/SPEC.md`, `docs/concepts_brief.md`, `docs/GOOGLE_SETUP.md`, `README.md`, `GLOSSARY.md`. Drift found during the check was fixed in the document where the code was the better answer. Those fixes are listed so the history is visible. What remains open is listed last.

## docs/SPEC.md

Found and fixed in the spec (code kept):
* Canvas limit: the spec said the limit is probed once with a "limit on this device is M MP" message. The worker checks each photo's own canvas by writing and reading back its last pixel and gives a different message. Spec updated.
* Path choice: the spec said `memory.grow`. The code probes with `ac_alloc` of `ac_full_bytes` and frees it, which the Rust allocator can reuse (pre-growing memory from JS would not be visible to the allocator). Spec updated.
* Worker restart and crash recovery live in a new `app/engine-client.js` (architecture pass). Drive state lives in `app/drive-config.js`. Neither was in the file list. Added.
* The engine also depends on `lightcraft-geom` (for the point curve's `Point`). Added.
* Site: the plan relied only on the generator marker. The code adds an `appPage` field to `projects.ts` so no placeholder page is rendered for AutoColor at all (the marker guard still protects the app). Added.
* Drive: 403 is treated like 401 (expired or revoked token). Added.
* Verification plan wording did not match the tests (per photo "into the band" and "1 point" clipping). Rewritten to the criteria the tests enforce.
* Test helpers, tuning examples, `tools/package.json` and the generated crate licence list were missing from the file list. Added.

Still open:
* `params::Choice::global_settings` is only used by a test. It is a thin alias of `render::global_only`.
* Banded path memory ("RGBA8 plus a fixed amount") is not measured with a number. A 12.6 MP photo ran on the banded path in desktop Chrome. Phones were not tested on real devices.
* Google Drive input is built to the spec but not tested end to end, because no Client ID, API key or project number exist yet. Only the "setup pending" state is tested.
* Fixed during this check: after "Reconnect Google Drive" retried photos were counted twice in the progress total. `queue.retry` now re runs them without growing the total (tested).

## docs/concepts_brief.md

* Says memory is probed with `memory.grow`. Superseded by the spec (allocation probe). The brief is kept as the brainstorm record.
* Says the hard cap includes "the browser's canvas limit" probed at start. Now checked per photo.

## docs/GOOGLE_SETUP.md

No drift detected. `config.js` line numbers (5, 7 and 9) match `app/config.js` and `site/projects/autocolor/config.js`.

## README.md

No drift detected.

## GLOSSARY.md

No drift detected.

## Legal and compliance check (scaled to a free static tool with no server)

AutoColor stores no personal data and has no backend. Photos are processed in the visitor's browser. Under India's DPDP Act 2023 and the GDPR the operator does not collect or process personal data in the usual sense. The Google Drive option is the one place where a third party (Google) is involved.

| # | Item | Status |
|---|---|---|
| 1 | Privacy policy | Short privacy note on the page ("Privacy and credits"): processed on device, never uploaded, no cookies, no analytics, Drive read only with permission and nothing stored. Scaled: enough for this tool. Linked as the consent screen's privacy policy URL. |
| 2 | Terms and conditions | Not a full document. The page says the tool is free and provided as is without warranty and that results may not suit every photo. Scaled: adequate for a free tool. |
| 3 | Cookie policy | Not needed: the page sets no cookies. Stated on the page. |
| 4 | Cookie consent | Not required for AutoColor itself (no cookies, no analytics, no third party requests until the visitor presses the Drive button). Google's sign in sets cookies on Google's own domains only after that click. **Flag:** confirm with a lawyer that no banner is needed for EU visitors who use the Drive button. |
| 5 | Consent on forms | No forms collect personal data. Drive access goes through Google's own consent screen. |
| 6 | What data is collected | Stated: nothing is collected. Drive files are read into the page only. |
| 7 | Why | Stated: Drive files are read only to correct them. |
| 8 | Deletion requests | Nothing is stored, so nothing to delete. The page links Google Account permissions to revoke access. |
| 9 | Access or correction requests | Same as 8. Harish's site has a contact section for questions. |
| 10 | Third party API terms | Google APIs Terms and the API Services User Data Policy (Limited Use) apply. The page states compliance and links Google's terms and privacy policy. Scope is `drive.file` only (non-sensitive). **Flag:** before moving the consent screen out of Testing, check Google's current verification and branding rules. |
| 11 | AI generated content and IP | AutoColor does not generate content. Its output is the user's own photo with tone and colour changes, so no new IP issue. The code was written with AI assistance. **Flag:** copyright in AI assisted code is unsettled law. The MIT licence is still the right choice. |
| 12 | Open source licences | LightCraft MIT OR Apache-2.0 (compatible with MIT). The Rust crate tree is mostly MIT or Apache-2.0, with some BSD-3-Clause, Zlib, IJG and Unicode-3.0 crates pulled in by LightCraft's codecs (likely removed by link time optimisation, not proven). All are permissive. `licenses/rust-crates.txt` lists every crate and licence. No JavaScript libraries. Build tool binaryen (Apache-2.0) is not shipped. |
| 13 | Copyrighted assets | None. Icons are hand drawn inline SVG. Fonts are system fonts. Test photos are CC0 (Wikimedia Commons) and sit in the repo only, not on the site. |
| 14 | Attribution | "Built on LightCraft by storytold (MIT/Apache-2.0)" with link, LightCraft's LICENSE-MIT, LICENSE-APACHE and NOTICE shipped in `licenses/`. CC0 photos are credited in `tests/fixtures/LICENSES.md` although CC0 does not require it. |
| 15 | UI assets licensing | Own inline SVG. The Drive button uses a generic outline icon, not Google's logo. **Flag:** Google has branding guidelines for Drive and sign in buttons. Check them before publishing the consent screen. |
| 16 | Age restrictions | None needed. Google accounts carry their own age rules. |
| 17 | High risk advice disclaimers | Not applicable (no financial, medical, legal or investment advice). |
| 18 | Refunds and cancellation | Not applicable (free, no payments). |
| 19 | Marketing claims | Accurate and tested: "never uploaded" (no upload code. Network use is the page itself and, on request, Google sign in, Picker and Drive downloads). "Changed only a little" for balanced photos is backed by the tests (mean deltaE00 at most 1.95 on 7 CC0 photos). No superlatives. |
| 20 | Final legal audit | Items for a lawyer: (a) EU cookie consent for the Drive sign in flow (item 4), (b) Google API terms and branding before leaving Testing (items 10 and 15), (c) copyright in AI assisted code (item 11), (d) two fixtures show identifiable people (people.jpg and beach.jpg). They are CC0 but personality rights are separate from copyright. They are used only as test data in the repo. Low risk, noted. |

## Round 2 drift note (2026-11-15)

Checked the Round 2 addendum in `SPEC.md` against the code.

* Readback probe: `app/readback.js` matches the spec (pattern, positions, `SAFE_AREA` 16,777,216, `NOISE_TOLERANCE` 32, messages). `worker.js` probes before the white fill. No drift.
* Stages: the worker sends reading, correcting and saving. `app.js` adds waiting and downloading and uses the bar values in the stage table. No drift.
* Queue: states, `start`, `remove`, `retry` into the current run and per run counts match. `size` counts the current run only, as specified.
* Page: Start button, overall and card bars (`role="progressbar"` with min, max, now, value text and a label), the `role="status"` live region and Remove match. One small extra: the status also reads "N photos ready. Press Start correcting." while photos wait. This is not in the spec and is harmless.
* Limits: the 50 MP, 40 MP, 12 MP and 4 MP limits are unchanged and only the 50 MP one rejects. The phone check only picks the path.
* Legal check: no change. The Start flow adds no data collection, cookies, network requests or third party assets. The new message names fingerprinting protection without naming any browser vendor.
