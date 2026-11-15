#!/usr/bin/env bash
# Browser checks with agent-browser against dist/ served at /projects/autocolor/.
# Usage: tests/browser/run.sh [session]
set -euo pipefail
cd "$(dirname "$0")/../.."
ROOT=$PWD
S=${1:-$(agent-browser session id --scope worktree --prefix task)}
PORT=8765
URL=http://127.0.0.1:$PORT/projects/autocolor/
DL=$(mktemp -d)
ab() { agent-browser --session "$S" "$@"; }
fail() { echo "FAIL: $*"; exit 1; }

node tests/browser/serve.mjs $PORT /projects/autocolor/=dist >/dev/null 2>&1 &
SERVER=$!
trap 'kill $SERVER 2>/dev/null' EXIT
sleep 1

# fixtures: damaged inputs (from samples/) and their undamaged originals (tests/fixtures/)
IN=$ROOT/samples
FX=$ROOT/tests/fixtures

ab set viewport 1280 900 >/dev/null
ab open "$URL" >/dev/null
ab errors --clear >/dev/null; ab console --clear >/dev/null
[ "$(ab get text '#drive')" = "Google Drive setup pending" ] || fail "Drive button text"
[ "$(ab eval "document.querySelector('#drive').disabled")" = "true" ] || fail "Drive button should be disabled"
echo "ok Drive button shows setup pending and is disabled"

wait_done() { ab wait --fn "document.querySelector('#status').textContent.startsWith('Done')" --timeout 120000 >/dev/null; }

# 1 photo
ab upload '#file' "$IN/lowlight_before.jpg" >/dev/null
wait_done
[ "$(ab get text '#status')" = "Done: 1 of 1 photos corrected" ] || fail "status after 1 photo: $(ab get text '#status')"
ab download '#results .download' "$DL/single.jpg" >/dev/null
python3 tests/browser/verify.py "$DL/single.jpg" "$IN/lowlight_before.jpg" "$FX/people.jpg"
[ -s "$DL/single.jpg" ] || fail "single download"
ab screenshot "$ROOT/screens/desktop-one.png" >/dev/null

# 3 photos and the zip
ab click '#clear' >/dev/null
ab upload '#file' "$IN/dark_before.jpg" "$IN/flat_before.jpg" "$IN/overexposed_before.jpg" >/dev/null
wait_done
[ "$(ab get text '#status')" = "Done: 3 of 3 photos corrected" ] || fail "status after 3 photos: $(ab get text '#status')"
ab click '#zip' >/dev/null
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

# a 12.6 MP photo goes through the banded path
if [ -f /tmp/ab-inputs-big.jpg ]; then
  ab click '#clear' >/dev/null
  ab upload '#file' /tmp/ab-inputs-big.jpg >/dev/null
  wait_done
  [ "$(ab eval "document.querySelector('#results .card').dataset.path")" = '"banded"' ] || fail "big photo path: $(ab get text '#results .state')"
  ab download '#results .download' "$DL/big.jpg" >/dev/null
  python3 -c "from PIL import Image; im = Image.open('$DL/big.jpg'); assert im.size == (2900, 4350), im.size; print('ok big photo banded, full size', im.size)"
fi

# unsupported files get a message, not a crash
printf 'not a photo' > "$DL/notes.txt"; cp "$IN/good_before.jpg" "$DL/raw.CR3"
ab click '#clear' >/dev/null
ab upload '#file' "$DL/raw.CR3" >/dev/null
ab wait --text "RAW files are not supported" --timeout 10000 >/dev/null && echo "ok RAW message"

# phone viewport
ab click '#clear' >/dev/null
ab set viewport 390 844 >/dev/null
ab upload '#file' "$IN/cast_before.jpg" >/dev/null
wait_done
ab screenshot --full "$ROOT/screens/mobile-390x844.png" >/dev/null
OVER=$(ab eval "document.documentElement.scrollWidth > window.innerWidth")
[ "$OVER" = "false" ] || fail "horizontal overflow at 390 px"
echo "ok no horizontal overflow at 390x844"

ERRS=$(ab errors 2>&1 || true)
CONS=$(ab console 2>&1 | grep -iE 'error|uncaught' || true)
[ -z "$(echo "$ERRS" | grep -viE '^(no errors|$)|no page errors' || true)" ] || fail "page errors: $ERRS"
[ -z "$CONS" ] || fail "console errors: $CONS"
echo "ok no page or console errors"
ab set viewport 1280 900 >/dev/null
ab a11y "$URL" 2>&1 | tail -5
echo "browser checks passed"
