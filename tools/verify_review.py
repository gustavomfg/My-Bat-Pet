#!/usr/bin/env python3
"""Verify actual Bevy review captures: python tools/verify_review.py DIRECTORY.

Requires Pillow for this development-only check; no runtime dependency.
"""
import sys
from pathlib import Path
from PIL import Image

files = sorted(Path(sys.argv[1]).glob('[0-9][0-9][0-9].png'))
if not files:
    raise SystemExit('No review frames found')
support = None
for path in files:
    with Image.open(path) as source:
        image = source.convert('RGB')
    assert image.size == (320, 320), f'{path}: unexpected window size'
    for y in range(0, 320, 8):
        for x in range(0, 320, 8):
            block = image.crop((x, y, x + 8, y + 8))
            assert all(low == high for low, high in block.getextrema()), (
                f'{path}: mixed logical pixel at ({x}, {y})'
            )
    current_support = image.crop((32, 8, 288, 48)).tobytes()
    if support is None:
        support = current_support
    assert current_support == support, f'{path}: claws moved'
print(f'{len(files)} frames: uniform 8x8 pixels and stationary support')
