import test from 'node:test';
import assert from 'node:assert/strict';
import { probeReadback, judgeReadback, SAFE_AREA } from '../../app/readback.js';

// A stand-in 2D context: keeps written pixels and returns them through `read(value, index)`,
// which can add noise (fingerprinting protection) or blank them (canvas too large).
function stubContext(read) {
  const px = new Map();
  let fill = [0, 0, 0];
  return {
    set fillStyle(v) {
      fill = [1, 3, 5].map((i) => parseInt(v.slice(i, i + 2), 16));
    },
    fillRect(x, y, w, h) {
      for (let j = y; j < y + h; j++) for (let i = x; i < x + w; i++) px.set(`${i},${j}`, [...fill, 255]);
    },
    getImageData(x, y, w, h) {
      const data = new Uint8ClampedArray(w * h * 4);
      for (let j = 0; j < h; j++)
        for (let i = 0; i < w; i++) (px.get(`${x + i},${y + j}`) ?? [0, 0, 0, 0]).forEach((v, c) => (data[(j * w + i) * 4 + c] = read(v, (j * w + i) * 4 + c)));
      return { data };
    },
  };
}

const W = 1280;
const H = 960;

test('an exact read back passes', () => {
  const diff = probeReadback(stubContext((v) => v), W, H);
  assert.equal(diff, 0);
  assert.equal(judgeReadback(W, H, diff), 'ok');
});

test('fingerprinting noise of 1 or 2 levels does not reject a 1.2 MP photo (the round 2 bug)', () => {
  for (const amp of [1, 2]) {
    const noisy = stubContext((v, i) => (i % 4 === 3 ? v : v + (i % 2 ? amp : -amp)));
    const diff = probeReadback(noisy, W, H);
    assert.ok(diff > 0, 'the stub really perturbs the read');
    assert.equal(judgeReadback(W, H, diff), 'ok');
  }
});

test('blank pixels above the safe area mean the canvas is too big', () => {
  const [w, h] = [5000, 4000];
  assert.ok(w * h > SAFE_AREA);
  const diff = probeReadback(stubContext(() => 0), w, h);
  assert.equal(judgeReadback(w, h, diff), 'too-big');
});

test('a blocked read below the safe area is reported as blocked, never as too big', () => {
  for (const read of [() => 0, () => 255, (v, i) => (i * 97) % 256]) {
    const diff = probeReadback(stubContext(read), W, H);
    assert.equal(judgeReadback(W, H, diff), 'blocked');
  }
  assert.equal(judgeReadback(4096, 4096, 255), 'blocked');
});

test('a 1x1 photo can be probed', () => {
  assert.equal(judgeReadback(1, 1, probeReadback(stubContext((v) => v), 1, 1)), 'ok');
});
