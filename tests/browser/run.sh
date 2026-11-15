#!/usr/bin/env bash
# Browser checks with agent-browser. By default against dist/ served locally at /projects/autocolor/.
# Usage: tests/browser/run.sh [session]
#   AUTOCOLOR_URL=https://harishkumartn.in/projects/autocolor/ tests/browser/run.sh   (no local server,
#   no screenshots of the new UI and no readback noise harness, which needs the local server)
set -euo pipefail
cd "$(dirname "$0")/../.."
ROOT=$PWD
S=${1:-$(agent-browser session id --scope worktree --prefix task)}
DL=$(mktemp -d)
ab() { agent-browser --session "$S" "$@"; }
fail() { echo "FAIL: $*"; exit 1; }
click() { ab scrollintoview "$1" >/dev/null; ab click "$1" >/dev/null; }

if [ -n "${AUTOCOLOR_URL:-}" ]; then
  URL=$AUTOCOLOR_URL
  LOCAL=
else
  PORT=8765
  URL=http://127.0.0.1:$PORT/projects/autocolor/
  LOCAL=1
  node tests/browser/serve.mjs $PORT /projects/autocolor/harness/=tests/browser/harness /projects/autocolor/=dist /photos/="$DL" >/dev/null 2>&1 &
  SERVER=$!
  trap 'kill $SERVER 2>/dev/null' EXIT
  sleep 1
fi
echo "testing $URL"

# fixtures: damaged inputs (from samples/) and their undamaged originals (tests/fixtures/)
IN=$ROOT/samples
FX=$ROOT/tests/fixtures
# a WhatsApp style photo: 1280x960 (1.2 MP) progressive JPEG, no EXIF, spaces and .jpeg in the name
WA=$(python3 tests/browser/make_whatsapp.py "$DL")
WA_ORIG=$DL/whatsapp-original.png

ab set viewport 1280 900 >/dev/null
ab open "$URL" >/dev/null
ab errors --clear >/dev/null; ab console --clear >/dev/null
[ "$(ab get text '#drive')" = "Google Drive setup pending" ] || fail "Drive button text"
[ "$(ab eval "document.querySelector('#drive').disabled")" = "true" ] || fail "Drive button should be disabled"
echo "ok Drive button shows setup pending and is disabled"
[ "$(ab eval "document.querySelector('#start').disabled")" = "true" ] || fail "Start should be disabled with no photos"
echo "ok Start is disabled with no photos"

wait_done() { ab wait --fn "document.querySelector('#status').textContent.startsWith('Done')" --timeout 120000 >/dev/null; }
# Start must be visible and enabled while photos wait, and nothing may run before it is pressed
# usage: expect_ready <ready cards> [finished cards from earlier runs]
expect_ready() {
  sleep 1.5
  local st
  st=$(ab eval "(() => { const b = document.querySelector('#start'), r = b.getBoundingClientRect(); return [b.disabled, r.width > 0 && r.height > 0, document.querySelectorAll('#results .card.done').length, document.querySelectorAll('#results .card.ready').length].join(' '); })()")
  [ "$st" = "\"false true ${2:-0} $1\"" ] || fail "ready state (disabled visible done ready) should be 'false true ${2:-0} $1': $st"
  echo "ok $1 photo(s) wait for Start and Start is visible"
}
# every visible bar is a progressbar at its max with a label, and the overall bar counts the run
expect_bars() {
  local st
  st=$(ab eval "(() => { const bars = [...document.querySelectorAll('[role=progressbar]:not([hidden])')]; return [bars.length, bars.every((b) => b.getAttribute('aria-valuenow') === b.getAttribute('aria-valuemax') && b.getAttribute('aria-valuemin') === '0' && (b.getAttribute('aria-label') ?? '').length > 0), document.querySelector('#overall').getAttribute('aria-valuetext')].join(' | '); })()")
  [ "$st" = "\"$(( $1 + 1 )) | true | $1 of $1 corrected\"" ] || fail "progress bars: $st"
  echo "ok $1 card bar(s) and the overall bar reached their max"
}
start() {
  click '#start'
  [ "$(ab eval "document.querySelector('#start').disabled")" = "true" ] || fail "Start should be disabled while running"
  wait_done
}

# 1 photo
ab upload '#file' "$IN/lowlight_before.jpg" >/dev/null
expect_ready 1
start
expect_bars 1
[ "$(ab get text '#status')" = "Done: 1 of 1 photos corrected" ] || fail "status after 1 photo: $(ab get text '#status')"
ab download '#results .download' "$DL/single.jpg" >/dev/null
python3 tests/browser/verify.py "$DL/single.jpg" "$IN/lowlight_before.jpg" "$FX/people.jpg"
[ -s "$DL/single.jpg" ] || fail "single download"
ab screenshot "$ROOT/screens/desktop-one.png" >/dev/null

# 3 photos (one more added and removed again) and the zip
click '#clear'
ab upload '#file' "$IN/dark_before.jpg" "$IN/flat_before.jpg" "$IN/overexposed_before.jpg" >/dev/null
ab upload '#file' "$IN/good_before.jpg" >/dev/null
expect_ready 4
click '#results .card:last-child .remove'
expect_ready 3
[ -z "$LOCAL" ] || ab screenshot --full "$ROOT/screens/start-ready-desktop.png" >/dev/null
start
expect_bars 3
[ "$(ab get text '#status')" = "Done: 3 of 3 photos corrected" ] || fail "status after 3 photos: $(ab get text '#status')"
click '#zip'
ab wait --download "$DL/photos.zip" --timeout 30000 >/dev/null
unzip -t "$DL/photos.zip" | grep -q "No errors detected" || fail "zip integrity"
mkdir -p "$DL/zip" && unzip -o -q "$DL/photos.zip" -d "$DL/zip"
ls "$DL/zip"
[ "$(ls "$DL/zip" | wc -l)" = 3 ] || fail "zip should hold 3 files"
python3 tests/browser/verify.py \
  "$DL/zip/dark_before-autocolor.jpg" "$IN/dark_before.jpg" "$FX/city.jpg" \
  "$DL/zip/flat_before-autocolor.jpg" "$IN/flat_before.jpg" "$FX/landscape.jpg" \
  "$DL/zip/overexposed_before-autocolor.jpg" "$IN/overexposed_before.jpg" "$FX/flowers.jpg"
ab screenshot --full "$ROOT/screens/desktop-three.png" >/dev/null
[ -z "$LOCAL" ] || ab screenshot --full "$ROOT/screens/start-done-desktop.png" >/dev/null

# photos added after a run wait for Start again and are counted in their own run
ab upload '#file' "$WA" >/dev/null
expect_ready 1 3
start
[ "$(ab get text '#status')" = "Done: 1 of 1 photos corrected" ] || fail "status after second run: $(ab get text '#status')"
ab download '#results .card:last-child .download' "$DL/wa-desktop.jpg" >/dev/null
python3 tests/browser/verify.py "$DL/wa-desktop.jpg" "$WA" "$WA_ORIG"
echo "ok WhatsApp style 1.2 MP JPEG corrected at desktop size"

# a 12.6 MP photo goes through the banded path
if [ -f /tmp/ab-inputs-big.jpg ]; then
  click '#clear'
  ab upload '#file' /tmp/ab-inputs-big.jpg >/dev/null
  start
  [ "$(ab eval "document.querySelector('#results .card').dataset.path")" = '"banded"' ] || fail "big photo path: $(ab get text '#results .state')"
  ab download '#results .download' "$DL/big.jpg" >/dev/null
  python3 -c "from PIL import Image; im = Image.open('$DL/big.jpg'); assert im.size == (2900, 4350), im.size; print('ok big photo banded, full size', im.size)"
fi

# unsupported files get a message, not a crash
printf 'not a photo' > "$DL/notes.txt"; cp "$IN/good_before.jpg" "$DL/raw.CR3"
click '#clear'
ab upload '#file' "$DL/raw.CR3" >/dev/null
ab wait --text "RAW files are not supported" --timeout 10000 >/dev/null && echo "ok RAW message"
[ "$(ab eval "document.querySelector('#start').disabled")" = "true" ] || fail "Start should stay disabled for a RAW file alone"

# phone viewport
click '#clear'
ab set viewport 390 844 >/dev/null
ab upload '#file' "$IN/cast_before.jpg" "$WA" >/dev/null
expect_ready 2
[ -z "$LOCAL" ] || ab screenshot "$ROOT/screens/start-ready-mobile.png" >/dev/null
H=$(ab eval "Math.round(document.querySelector('#start').getBoundingClientRect().height)")
[ "$H" -ge 44 ] || fail "Start tap target is $H px"
start
expect_bars 2
[ "$(ab get text '#status')" = "Done: 2 of 2 photos corrected" ] || fail "status on phone: $(ab get text '#status')"
ab download '#results .card:last-child .download' "$DL/wa-phone.jpg" >/dev/null
python3 tests/browser/verify.py "$DL/wa-phone.jpg" "$WA" "$WA_ORIG"
echo "ok WhatsApp style 1.2 MP JPEG corrected at 390x844"
ab screenshot --full "$ROOT/screens/mobile-390x844.png" >/dev/null
OVER=$(ab eval "document.documentElement.scrollWidth > window.innerWidth")
[ "$OVER" = "false" ] || fail "horizontal overflow at 390 px"
echo "ok no horizontal overflow at 390x844"

ERRS=$(ab errors 2>&1 || true)
CONS=$(ab console 2>&1 | grep -iE 'error|uncaught' || true)
[ -z "$(echo "$ERRS" | grep -viE '^(no errors|$)|no page errors' || true)" ] || fail "page errors: $ERRS"
[ -z "$CONS" ] || fail "console errors: $CONS"
echo "ok no page or console errors"

# canvas readback noise (fingerprinting protection) in the real worker: noise is accepted and does not
# show in the output. Blocked reads get a clear message. Needs the local server (harness/ is not shipped).
if [ -n "$LOCAL" ]; then
  ab set viewport 1280 900 >/dev/null
  WA_URL="/photos/$(basename "$WA" | sed 's/ /%20/g')"
  for q in amp=1 amp=2 blocked=1; do
    ab open "${URL}harness/?photo=$WA_URL&$q" >/dev/null
    ab wait --fn "document.title === 'done'" --timeout 120000 >/dev/null
    R=$(ab eval "JSON.stringify(window.result)")
    echo "$q: $R"
    python3 - "$q" "$R" <<'PY' || fail "readback harness $q"
import json, sys
q, r = sys.argv[1], json.loads(json.loads(sys.argv[2]))
if q.startswith("blocked"):
    assert not r["ok"] and "blocking access to the photo pixels" in r["message"], r
else:
    assert r["ok"] and r["w"] * r["h"] == 1280 * 960, r
    assert r["meanDiff"] < 2.5 and r["psnr"] > 38, r  # not visible
    assert abs(r["choice"]["exposure"][0] - r["choice"]["exposure"][1]) <= 0.05, r
print(f"ok readback {q}")
PY
  done
  ab open "$URL" >/dev/null
fi

ab set viewport 1280 900 >/dev/null
ab a11y "$URL" 2>&1 | tail -5
echo "browser checks passed"
