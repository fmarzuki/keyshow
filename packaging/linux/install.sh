#!/usr/bin/env sh
# Install Keyshow for the current user, plus (with sudo) a udev rule that lets
# the logged-in user read keyboards and mice.
#
#   packaging/linux/install.sh              install binary, launcher, icon, udev rule
#   packaging/linux/install.sh --no-udev    skip the udev rule (e.g. you use the input group)
#   packaging/linux/install.sh --uninstall  remove everything again
#
# Run it as your normal user; it calls sudo only for the udev rule.
set -eu

here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." 2>/dev/null && pwd || echo "$here")
bin_dir="$HOME/.local/bin"
app_dir="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
icon_dir="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/scalable/apps"
rule=/etc/udev/rules.d/70-keyshow.rules

if [ "$(id -u)" = 0 ]; then
    echo "Run this as your normal user (not with sudo); it asks for sudo when needed." >&2
    exit 1
fi

reload_udev() {
    sudo udevadm control --reload
    sudo udevadm trigger --subsystem-match=input --action=change
}

if [ "${1:-}" = "--uninstall" ]; then
    "$bin_dir/keyshow" --quit 2>/dev/null || true
    rm -f "$bin_dir/keyshow" "$app_dir/keyshow.desktop" "$icon_dir/keyshow.svg"
    if [ -e "$rule" ]; then
        sudo rm -f "$rule"
        reload_udev
    fi
    echo "Keyshow removed. Settings remain in ${XDG_CONFIG_HOME:-$HOME/.config}/keyshow."
    exit 0
fi

# Files sit next to this script in a release archive, or in the repo layout.
find_file() {
    for f in "$here/$1" "$root/$2"; do
        [ -f "$f" ] && { echo "$f"; return; }
    done
}
binary=$(find_file keyshow target/release/keyshow)
if [ -z "$binary" ]; then
    echo "No keyshow binary found. Build it first:  cargo build --release" >&2
    exit 1
fi

install -Dm755 "$binary" "$bin_dir/keyshow"
sed "s|^Exec=keyshow|Exec=$bin_dir/keyshow|" "$(find_file keyshow.desktop packaging/linux/keyshow.desktop)" \
    > "$bin_dir/.keyshow.desktop.tmp"
install -Dm644 "$bin_dir/.keyshow.desktop.tmp" "$app_dir/keyshow.desktop"
rm -f "$bin_dir/.keyshow.desktop.tmp"
install -Dm644 "$(find_file keyshow.svg packaging/linux/keyshow.svg)" "$icon_dir/keyshow.svg"
echo "Installed $bin_dir/keyshow and the Keyshow launcher."

if [ "${1:-}" != "--no-udev" ]; then
    echo "Installing udev rule $rule (needs sudo)..."
    sudo install -Dm644 "$(find_file 70-keyshow.rules packaging/linux/70-keyshow.rules)" "$rule"
    reload_udev
    echo "Done. Keyboards and mice are now readable by the logged-in user."
fi

echo "Start it from your app menu, or run: keyshow"
