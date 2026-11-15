"""Checks downloaded outputs: valid JPEGs at the input size, different from the input and with a lower
luminance error (mean per pixel |L* difference| to the undamaged original) than the input had. Usage: verify.py out.jpg input.jpg original.jpg ..."""
import sys
from PIL import Image, ImageChops, ImageStat


def l_error(a, b):
    """Mean per pixel |L*(a) - L*(b)| (L* 0..100)."""
    la, lb = a.convert("RGB").convert("LAB").getchannel("L"), b.convert("RGB").convert("LAB").getchannel("L")
    return ImageStat.Stat(ImageChops.difference(la, lb)).mean[0] * 100 / 255


args = sys.argv[1:]
ok = True
for out_path, in_path, orig_path in zip(args[0::3], args[1::3], args[2::3]):
    out = Image.open(out_path)
    out.verify()
    out = Image.open(out_path)
    src, orig = Image.open(in_path), Image.open(orig_path)
    diff = ImageStat.Stat(ImageChops.difference(out.convert("RGB"), src.convert("RGB"))).mean
    before, after = l_error(src, orig), l_error(out, orig)
    good = out.format == "JPEG" and out.size == src.size and sum(diff) > 1 and after < before
    ok &= good
    print(f"{'ok ' if good else 'BAD'} {out_path}: {out.format} {out.size}, differs from input by {sum(diff)/3:.1f} levels, luminance error {before:.1f} -> {after:.1f}")
sys.exit(0 if ok else 1)
