#!/usr/bin/env bash
# Usage: bundle-macos.sh <binary-path> <version> <out-dir> [sign]
#
# Default: bundle only (unsigned dmg). Pass "sign" as 4th arg to also
# codesign + notarize + staple; requires the env vars below.
set -euo pipefail

bin="$1"
version="$2"
out="$3"
mode="${4:-bundle}"
app_name="Wnav"
bundle_id="app.wnav.Wnav"
app_dir="$out/${app_name}.app"
contents_macos="$app_dir/Contents/MacOS"
resources="$app_dir/Contents/Resources"

if [ ! -x "$bin" ]; then
    echo "bundle: $bin not found/executable" >&2
    exit 1
fi

mkdir -p "$contents_macos" "$resources"
cp "$bin" "$contents_macos/$app_name"

# Icon: render SVG -> png set -> icns (iconutil is present on GH macos runners).
if ! command -v rsvg-convert >/dev/null 2>&1 && ! command -v magick >/dev/null 2>&1; then
    brew install librsvg >/dev/null 2>&1 || true
fi
if command -v rsvg-convert >/dev/null 2>&1 || command -v magick >/dev/null 2>&1; then
    bash scripts/release/export-brand.sh
    iconset="$out/icon.iconset"
    mkdir -p "$iconset"
    for exp in 4 5 6 7 8 9; do
        size=$((2 ** exp))
        if command -v rsvg-convert >/dev/null 2>&1; then
            rsvg-convert -w "$size" brand/icon.svg > "$iconset/icon_${size}x${size}.png"
            rsvg-convert -w $((size * 2)) brand/icon.svg > "$iconset/icon_${size}x${size}@2x.png"
        else
            magick -background none brand/icon.svg -resize "$size" "$iconset/icon_${size}x${size}.png"
            magick -background none brand/icon.svg -resize "$((size * 2))" "$iconset/icon_${size}x${size}@2x.png"
        fi
    done
    iconutil -c icns "$iconset" -o "$resources/AppIcon.icns"
    rm -rf "$iconset"
else
    echo "bundle: no rasterizer available, shipping without icon" >&2
fi

cat > "$app_dir/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key><string>${app_name}</string>
    <key>CFBundleIdentifier</key><string>${bundle_id}</string>
    <key>CFBundleName</key><string>${app_name}</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>${version}</string>
    <key>CFBundleVersion</key><string>${version}</string>
    <key>CFBundleIconFile</key><string>AppIcon</string>
    <key>LSMinimumSystemVersion</key><string>13.0</string>
    <key>LSUIElement</key><true/>
    <key>NSHumanReadableCopyright</key><string>Wnav contributors</string>
</dict>
</plist>
PLIST

sign() {
    security create-keychain -p "$KEYCHAIN_PASSWORD" build.keychain
    security set-key-partition-list -S apple-tool:,apple: -s -k "$KEYCHAIN_PASSWORD" build.keychain
    security default-keychain -s build.keychain
    security import "$CERT_PATH" -k build.keychain -P "$CERT_PASSWORD" -T /usr/bin/codesign
    codesign --force --deep --options runtime --timestamp \
        --sign "$SIGNING_IDENTITY" "$app_dir"
    xcrun notarytool store-credentials wnav-notary --apple-id "$APPLE_ID" \
        --team-id "$APPLE_TEAM_ID" --password "$APPLE_PASSWORD"
    dmg_unsigned="$out/${app_name}-${version}-unsigned.dmg"
    hdiutil create -volname "$app_name" -srcfolder "$app_dir" -ov -format UDZO "$dmg_unsigned"
    xcrun notarytool submit "$dmg_unsigned" --keychain-profile wnav-notary --wait
    xcrun stapler staple "$dmg_unsigned"
    mv "$dmg_unsigned" "$out/wnav-${version}-aarch64-apple-darwin.dmg"
    security delete-keychain build.keychain
    echo "bundle: signed + notarized dmg"
}

case "$mode" in
    sign)
        for var in CERT_PATH CERT_PASSWORD SIGNING_IDENTITY APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID KEYCHAIN_PASSWORD; do
            if [ -z "${!var:-}" ]; then
                echo "bundle: sign mode requires $var" >&2
                exit 1
            fi
        done
        sign
        ;;
    bundle)
        echo "bundle: bundle-only mode — shipping UNSIGNED dmg" >&2
        hdiutil create -volname "$app_name" -srcfolder "$app_dir" -ov -format UDZO \
            "$out/wnav-${version}-aarch64-apple-darwin.dmg"
        ;;
    *)
        echo "bundle: unknown mode '$mode' (use 'bundle' or 'sign')" >&2
        exit 2
        ;;
esac
