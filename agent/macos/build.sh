#!/bin/sh
# Builds aridan-presence.app: the Rust side (src/) as a static library, the
# SwiftUI app (Sources/) linked against it, and the two put together into an
# app bundle with its icon, signed for this Mac.
#
#   ./build.sh            builds macos/build/aridan-presence.app
#   ./build.sh install    and copies it to /Applications, quitting any copy
#                         that is running first
#
# Needs Xcode (or its command line tools) for macOS 26 or later, and Rust.

set -e

here=$(cd "$(dirname "$0")" && pwd)
agent="$here/.."
app="$here/build/aridan-presence.app"

version=$(sed -n 's/^version = "\(.*\)"/\1/p' "$here/Cargo.toml" | head -1)

echo "Building the Rust side..."
(cd "$agent" && cargo build --release -p aridan-presence-macos)

echo "Building the app..."
(cd "$here" && swift build -c release --product Presence)
binary="$(cd "$here" && swift build -c release --show-bin-path)/Presence"

rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"

cp "$binary" "$app/Contents/MacOS/aridan-presence"
cp "$agent/icons/icon.icns" "$app/Contents/Resources/icon.icns"
sed "s/VERSION/$version/g" "$here/Info.plist" > "$app/Contents/Info.plist"

# Signed for this Mac only ("-" is no identity). Enough for it to run here,
# and for start at login to work.
codesign --force --deep --sign - "$app"

echo "Built $app"

if [ "$1" = "install" ]; then
    # Quitting it the polite way lets it clear itself from the site first.
    if pgrep -x aridan-presence >/dev/null; then
        osascript -e 'tell application id "net.aridan.presence" to quit' || true
        sleep 2
    fi

    rm -rf /Applications/aridan-presence.app
    cp -R "$app" /Applications/
    echo "Installed to /Applications. Open it from Launchpad or Spotlight."
fi
