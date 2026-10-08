#!/bin/sh
# Installs aridan-presence for this user only, the way an app from the
# package manager would be, just under ~/.local instead of /usr:
#
#   the program     ~/.local/bin/aridan-presence
#   the menu entry  ~/.local/share/applications/aridan-presence.desktop
#   the icon        ~/.local/share/icons/hicolor/.../aridan-presence.png
#
# This is the Qt version (../qt), which the desktop's theme draws. Run from
# the agent folder after:
#
#   cargo build --release -p aridan-presence-qt
#
# Start at login is switched on from the app's Options menu or Settings.

set -e

here=$(dirname "$0")
agent="$here/.."
data="${XDG_DATA_HOME:-$HOME/.local/share}"

# A running copy keeps the old program file busy, and would carry on running
# the old version. Quitting it the polite way lets it clear itself from the
# site first.
if pkill -TERM -x aridan-presence; then
    sleep 2
fi

install -Dm755 "$agent/target/release/aridan-presence-qt" "$HOME/.local/bin/aridan-presence"
install -Dm644 "$here/aridan-presence.desktop" "$data/applications/aridan-presence.desktop"
install -Dm644 "$agent/icons/32x32.png" "$data/icons/hicolor/32x32/apps/aridan-presence.png"
install -Dm644 "$agent/icons/128x128.png" "$data/icons/hicolor/128x128/apps/aridan-presence.png"
install -Dm644 "$agent/icons/128x128@2x.png" "$data/icons/hicolor/256x256/apps/aridan-presence.png"
install -Dm644 "$agent/icons/icon.png" "$data/icons/hicolor/512x512/apps/aridan-presence.png"

# So the menu picks up the new entry and icon without logging out.
update-desktop-database "$data/applications" 2>/dev/null || true
gtk-update-icon-cache -q "$data/icons/hicolor" 2>/dev/null || true

echo "Installed. Start it from the app menu, or run: aridan-presence"
