"""Writes a WhatsApp-style test photo: about 1.2 MP, progressive JPEG, quality 80, 4:2:0, no EXIF,
named like WhatsApp names its images (spaces and a .jpeg extension). Made from a CC0 fixture.
Also writes the undamaged original (whatsapp-original.png) for verify.py.
Usage: make_whatsapp.py <out dir> -> prints the path"""
import sys
from pathlib import Path
from PIL import Image

fixtures = Path(__file__).resolve().parent.parent / "fixtures"
out = Path(sys.argv[1]) / "WhatsApp Image 2026-11-15 at 2.56.11 PM.jpeg"
im = Image.open(fixtures / "food.jpg").convert("RGB").resize((1280, 960), Image.LANCZOS)
im.save(out.with_name("whatsapp-original.png"))
# a little underexposed with a warm cast, like a phone photo under indoor light
im = Image.merge("RGB", [c.point(lambda v, k=k: int(v * k)) for c, k in zip(im.split(), (0.8, 0.7, 0.55))])
im.save(out, "JPEG", quality=80, progressive=True, subsampling=2, optimize=True)
print(out)
