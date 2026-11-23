#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

APP="$ROOT/dist/GeoSignal Local SEO.app"
BIN="$ROOT/target/release/geosignal-desktop"

echo "Building GeoSignal desktop (release)…"
cargo build --release --bin geosignal-desktop

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

cp "$BIN" "$APP/Contents/MacOS/GeoSignal"
cp -R "$ROOT/public" "$APP/Contents/Resources/public"
cp "$ROOT/LICENSE" "$APP/Contents/Resources/LICENSE"
cp "$ROOT/NOTICE" "$APP/Contents/Resources/NOTICE"
cp "$ROOT/THIRD_PARTY_LICENSES.md" "$APP/Contents/Resources/THIRD_PARTY_LICENSES.md"
cp "$ROOT/MODEL_LICENSES.md" "$APP/Contents/Resources/MODEL_LICENSES.md"
cp -R "$ROOT/licenses" "$APP/Contents/Resources/licenses"

cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>GeoSignal Local SEO</string>
  <key>CFBundleDisplayName</key><string>GeoSignal Local SEO</string>
  <key>CFBundleIdentifier</key><string>com.geosignal.desktop</string>
  <key>CFBundleVersion</key><string>1.0.0</string>
  <key>CFBundleShortVersionString</key><string>1.0.0</string>
  <key>CFBundleExecutable</key><string>GeoSignal</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.developer-tools</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSPrincipalClass</key><string>NSApplication</string>
  <key>NSAppTransportSecurity</key><dict><key>NSAllowsLocalNetworking</key><true/></dict>
</dict>
</plist>
PLIST

codesign --force --deep --sign - "$APP" >/dev/null 2>&1 || true

echo "Built $APP"
echo "Launch with: open \"$APP\""
