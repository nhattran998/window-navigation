# brand/

Source-of-truth vector assets for Wnav's identity. Everything binary
(`.icns`, `.ico`, PNG exports) is generated from these SVGs — never commit
exports alone.

```
brand/
  icon.svg                    master icon (512 tile, snap-a-window motif)
  macos/
    dmg-background.svg        DMG installer background (1024x320, 1x)
    README.md                 export pipeline + sizes
  windows/
    icon.svg                  Windows tile variant (flatter, 48px radius)
  linux/
    icon.svg                  Linux hicolor variant (circular plate)
```

## Motif

A window snapped to a grid position: dimmed full-window outline, the snapped
half filled with the accent blue (`#0a84ff` macOS / `#0078d4` Windows /
`#3584e4` GNOME), an arrow hinting the snap direction. Monochrome-friendly;
never rely on color alone (the arrow carries the meaning in grayscale).
