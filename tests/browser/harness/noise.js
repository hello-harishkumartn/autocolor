// Imitates canvas fingerprinting protection (Brave farbling, Firefox fingerprinting protection, Safari
// advanced fingerprinting protection): getImageData results are perturbed before the page sees them.
// ?amp=N adds +-N (cycling -N..N) to every colour value; ?blocked=1 returns opaque white like a blocked read.
const q = new URL(import.meta.url).searchParams;
const amp = Number(q.get('amp') ?? 0);
const blocked = q.get('blocked') === '1';
const real = OffscreenCanvasRenderingContext2D.prototype.getImageData;
OffscreenCanvasRenderingContext2D.prototype.getImageData = function (...args) {
  const img = real.apply(this, args);
  const d = img.data;
  for (let i = 0; i < d.length; i++) {
    if (blocked) d[i] = 255;
    else if (amp && i % 4 !== 3) d[i] += ((i * 7 + (i >> 2)) % (2 * amp + 1)) - amp || amp;
  }
  return img;
};
