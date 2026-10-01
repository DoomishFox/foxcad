#!/bin/sh
# Development bundle, using the debug executable produced by cargo build.
set -eu
cd "$(dirname "$0")/.."
cargo build --locked
bundle="$PWD/target/FoxCAD.app"
mkdir -p "$bundle/Contents/MacOS"
cp target/debug/foxcad "$bundle/Contents/MacOS/FoxCAD"
cat > "$bundle/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>FoxCAD</string>
<key>CFBundleIdentifier</key><string>dev.foxcad.phase0</string>
<key>CFBundleName</key><string>FoxCAD</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleVersion</key><string>0.0.1</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
printf 'Development bundle: %s\n' "$bundle"
