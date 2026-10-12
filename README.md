<p align="center">
  <img src="packaging/linux/keyshow.svg" alt="Keyshow icon" width="96" height="96">
</p>

<h1 align="center">Keyshow</h1>

<p align="center">
  <img src="keyshow.gif" alt="Keyshow showing keystrokes and shortcuts on screen" width="624">
</p>

Keyshow shows the keys and shortcuts you press, and optionally your mouse clicks, on screen.
It is made for tutorials, software demos, presentations and screen recordings. Inspired by
[KeyScreen](https://keyscreen.app) (macOS), it is written in Rust for **Linux and Windows**.

## Features

- The overlay is transparent, click-through and always on top, so it never gets in your way.
- **Display modes**
  - *Shortcuts only* (the default): Ctrl/Alt/Super + key combinations, plus function and special keys.
  - *All keys*: everything you type. Plain typing is merged into one running text bubble, and Backspace edits it.
- **Filters:** function keys (F1–F12), special keys (Enter, Esc, Tab, arrows, …), modifiers pressed on their own, and mouse clicks and scrolling.
- **Repeats** are merged into **×N**, for example `Ctrl + Z ×3`, or `Left Click ×2` for a double click.
- **Themes:** 5 built-in themes (Dark, Light, Neon, Minimal, Solarized) plus a **Custom** theme. The custom theme sets the text, background, border and counter colors, the corner radius and the shadow.
- **Key styles:** **Pill** (`Ctrl + Shift + T`) or **Keycaps** (each key drawn as its own keycap). Labels can be text (`Ctrl Shift`) or symbols (`⌃ ⇧ ⌥ ⏎ ⌫`).
- **Adjustable:** font size, opacity, display duration, fade-out and how many bubbles of history to keep.
- **Placement:** 9 anchor positions plus margins, and a monitor picker for multi-monitor setups.
- **Global shortcuts**
  - `Ctrl+Alt+Shift+K` pauses and resumes. Use it before typing a password.
  - `Ctrl+Alt+Shift+S` opens Settings.
- **Single instance:** launching `keyshow` again opens Settings in the running instance. `keyshow --pause` and `keyshow --quit` control it too.
- **Windows tray icon** with Settings, Pause and Quit.
- **Settings are saved automatically** to `~/.config/keyshow/config.json` on Linux and `%APPDATA%\keyshow\config.json` on Windows.

## Privacy

Everything runs locally. Keyshow **never stores or sends** your keystrokes anywhere: it keeps no logs and makes no network connections.

Unlike macOS, Linux and Windows don't tell applications when you are typing into a password field. **Pause Keyshow (`Ctrl+Alt+Shift+K`) before typing a password** while recording.

## Installation

### Linux: .deb package (Ubuntu 22.04+, Debian 12+)

```sh
sudo apt install ./keyshow_0.1.0-1_amd64.deb
```

The package installs `/usr/bin/keyshow`, a launcher, an icon and the udev rule `/usr/lib/udev/rules.d/70-keyshow.rules`, and applies the rule right away. Keyshow works immediately, without logging out. Remove it with `sudo apt remove keyshow`.

### Linux: from source (requires Rust)

```sh
cargo build --release
packaging/linux/install.sh
```

Run `install.sh` as your normal user. It installs:
- the binary to `~/.local/bin/keyshow`
- a launcher and icon in the app menu
- the udev rule `/etc/udev/rules.d/70-keyshow.rules`, using `sudo`

To remove everything again, run `packaging/linux/install.sh --uninstall`.

**Why the udev rule?** On Wayland, ordinary applications can't see keys pressed in other windows. Keyshow therefore reads keyboards and mice directly from `/dev/input`, which works the same on X11 and Wayland. The udev rule gives the user logged in at the local seat read access to keyboards, mice and touchpads, through a `uaccess` ACL. No logout is needed.

The alternative is to add yourself to the `input` group with `sudo usermod -aG input $USER`, log out and back in, then run `install.sh --no-udev`.

> ⚠️ Either way, **any program you run** can read every keystroke. That is the cost of showing keystrokes on Wayland.

On GNOME and KDE Wayland sessions, the overlay runs through **XWayland**, because Wayland doesn't let applications position their windows or keep them on top. XWayland is enabled by default on Ubuntu, Fedora and most other distributions.

### Windows

Run `keyshow-0.1.0-windows-x86_64-setup.exe`. It installs per user, so it needs no administrator rights, into `%LOCALAPPDATA%\Programs\Keyshow`. You get:
- a Start menu shortcut
- an optional desktop shortcut
- an optional *start when I sign in*
- an *Apps & features* entry for uninstalling

To run Keyshow without installing it, use `keyshow-0.1.0-windows-x86_64-portable.exe`.

The installer isn't code-signed yet, so Windows SmartScreen may show a warning. Click *More info*, then *Run anyway*.

No special permissions are needed: Keyshow uses Windows' low-level keyboard and mouse hooks. Keys typed into windows running as Administrator are only visible if Keyshow also runs as Administrator.

## Usage

```
keyshow              start (or open Settings if already running)
keyshow --settings   open Settings
keyshow --pause      pause / resume
keyshow --quit       quit
keyshow --demo       show sample keystrokes instead of reading real input
```

On the first run, the Settings window opens automatically. Closing it leaves the overlay running.

## Known limitations

- **Linux:** character keys are labelled for a **US** layout, because evdev only reports physical key codes. On Windows, typed text follows the active layout.
- **Linux:** touchpad tap-to-click isn't shown, only physical clicks. Taps are handled by libinput inside the compositor.
- **Linux:** with XWayland disabled, the overlay can't position itself or stay on top.
- Fullscreen applications (videos, or games in F11 mode) may cover the overlay, depending on the window manager.
- **Windows:** the overlay only appears on the virtual desktop Keyshow was started on, because Windows has no public API for "show on all desktops". On Linux, the overlay appears on every workspace.
- There is no tray icon on Linux, because GNOME hides tray icons. Use the launcher, `keyshow --settings` or `Ctrl+Alt+Shift+S` instead.

## Building packages

Only Docker is needed. The `.deb` is built on Debian 12. The Windows build is cross-compiled with mingw-w64 and packaged with NSIS:

```sh
packaging/build-packages.sh          # everything → dist/
packaging/build-packages.sh deb      # .deb only
packaging/build-packages.sh windows  # installer + portable .exe only
```

GitHub Actions (`.github/workflows/build.yml`) does the same and attaches the results to a Release for every `v*` tag.

To regenerate the Windows icon (`packaging/windows/keyshow.ico`), run `python3 packaging/icons.py`.

## Development

```sh
cargo test                                      # display logic & config
cargo clippy --all-targets
cargo check --target x86_64-pc-windows-msvc     # check the Windows code from Linux
cargo run -- --demo
```

Layout:

| File | Contents |
|---|---|
| `src/display.rs` | core logic: modifiers, modes, ×N, running text, hotkeys, fading (+ unit tests) |
| `src/overlay.rs` | bubble drawing |
| `src/settings.rs` | Settings window |
| `src/input/linux.rs` | evdev backend (hotplug, never grabs devices) |
| `src/input/windows.rs` | `WH_KEYBOARD_LL` / `WH_MOUSE_LL` hooks + `ToUnicodeEx` |
| `src/app.rs` | eframe app: overlay placement, monitors, always-on-top |
| `packaging/linux/` | udev rule, `.desktop` launcher, icon, `install.sh` |
| `packaging/windows/` | NSIS installer script, icon, application manifest |
