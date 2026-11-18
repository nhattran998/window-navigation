# macOS brand assets

- `dmg-background.svg` — DMG installer background at 1x (1024x320).
  Export pipeline: `rsvg-convert -w 1024 -h 320 dmg-background.svg > background.png`
  and and `-w 2048 -h 640` for the retina 2x export.
- Master `../icon.svg` renders to the `.icns` set: 16, 32, 64, 128, 256, 512, 1024.

Generate with `just brand-export` once ImageMagick/rsvg-convert is available;
until then the SVGs are the distribution source.
