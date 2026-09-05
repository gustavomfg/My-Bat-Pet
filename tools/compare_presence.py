#!/usr/bin/env python3
"""Compose review screenshots, with the received cursor marked for context.

python tools/compare_presence.py BEFORE AFTER OUTPUT_DIRECTORY
Requires Pillow. Does not modify game assets or the source captures.
"""
import csv
import math
from pathlib import Path
import sys
from PIL import Image, ImageDraw, ImageFont

before, after, output = map(Path, sys.argv[1:4])
output.mkdir(parents=True, exist_ok=True)
try:
    font = ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf', 12)
except OSError:
    font = ImageFont.load_default()

def traces(directory):
    with (directory / 'cursor.csv').open() as source:
        return {int(row['frame']): row for row in csv.DictReader(source)}

records = [traces(before), traces(after)]
frames = []
for source in sorted(before.glob('[0-9][0-9][0-9].png')):
    index = int(source.stem)
    if not (after / source.name).exists():
        continue
    canvas = Image.new('RGB', (640, 384), (27, 22, 35))
    draw = ImageDraw.Draw(canvas)
    for side, directory in enumerate([before, after]):
        with Image.open(directory / source.name) as capture:
            canvas.paste(capture.convert('RGB'), (side * 320, 32))
        draw.text((side * 320 + 16, 9), ['ANTES · base oficial', 'DEPOIS · presença'][side], font=font, fill='#eee2fa')
        row = records[side].get(index)
        if row:
            x, y = float(row['cursor_x']), float(row['cursor_y'])
            if math.isfinite(x) and math.isfinite(y) and 0 <= x < 320 and 0 <= y < 320:
                x, y = round(x) + side * 320, round(y) + 32
                draw.line((x-4,y,x+4,y), fill='#9aebce', width=1)
                draw.line((x,y-4,x,y+4), fill='#9aebce', width=1)
    t = float(records[1][index]['time'])
    phase = ('repouso' if t < 2 else 'aproximação' if t < 4 else
             'acompanhar' if t < 8 else 'cursor parado · relaxar' if t < 13 else
             'afastamento' if t < 15 else 'ausência · quietude' if t < 22 else
             'retorno breve' if t < 23 else 'último olhar · repouso')
    draw.text((16, 362), f'{t:04.1f}s  |  {phase}', font=font, fill='#eee2fa')
    draw.text((460, 362), '+ cursor recebido', font=font, fill='#9aebce')
    frames.append(canvas)

assert frames, 'No matching review captures'
frames[0].save(output / 'presence-before-after.gif', save_all=True,
    append_images=frames[1:], duration=83, loop=0, disposal=2)
sheet = Image.new('RGB', (640, 384 * 4))
for row, seconds in enumerate([4.0, 8.0, 12.0, 15.8]):
    sheet.paste(frames[min(len(frames)-1, round((seconds-0.5)*12))], (0, row*384))
sheet.save(output / 'presence-poses.png')
print(f'{len(frames)} paired captures written to {output}')
