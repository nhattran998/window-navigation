#!/usr/bin/env bash
# Export brand SVGs to platform rasters (.icns/.ico source PNGs).
# Requires rsvg-convert (brew: librsvg) or falls back to ImageMagick.
set -euo pipefail
cd "$(dirname "$0")/../.."

render() {
    local svg="$1" width="$2" out="$3"
    if command -v rsvg-convert >/dev/null 2>&1; then
        rsvg-convert -w "$width" "$svg" > "$out"
    elif command -v magick >/dev/null 2>&1; then
        magick -background none "$svg" -resize "$width" "$out"
    else
        echo "export: need rsvg-convert or magick" >&2
        exit 1
    fi
}

mkdir -p brand/exports
render brand/icon.svg 1024 brand/exports/icon-1024.png
render brand/icon.svg 512  brand/exports/icon-512.png
render brand/icon.svg 256  brand/exports/icon-256.png
render brand/icon.svg 128  brand/exports/icon-128.png
render brand/macos/dmg-background.svg 1024 brand/exports/dmg-background.png
render brand/macos/dmg-background.svg 2048 brand/exports/dmg-background-2x.png
echo "export: wrote brand/exports/"
