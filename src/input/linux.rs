//! Linux backend: reads keyboards and mice straight from evdev (`/dev/input/event*`).
//! This works on both X11 and Wayland, but needs read access to the devices
//! (see packaging/linux/70-keyshow.rules). Devices are never grabbed, so input
//! still reaches every other application.

use std::collections::HashSet;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use evdev::{Device, EventSummary, KeyCode, RelativeAxisCode};

use super::{InputEvent, InputStatus, MouseButton, Sink};
use crate::keys::Key;

const RESCAN_INTERVAL: Duration = Duration::from_secs(3);

pub fn spawn(sink: Sink) {
    thread::Builder::new()
        .name("keyshow-evdev-scan".into())
        .spawn(move || scan_loop(sink))
        .expect("spawn evdev scanner");
}

fn scan_loop(sink: Sink) {
    // Paths with a live reader thread.
    let active: Arc<Mutex<HashSet<PathBuf>>> = Arc::default();
    // Paths we opened and found to be neither keyboard nor mouse.
    let mut ignored: HashSet<PathBuf> = HashSet::new();
    let mut last_status = None;

    loop {
        let mut denied = false;
        let mut paths: Vec<PathBuf> = match std::fs::read_dir("/dev/input") {
            Ok(rd) => rd
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.starts_with("event"))
                })
                .collect(),
            Err(e) => {
                sink.send(InputEvent::Status(InputStatus::Failed(format!(
                    "Cannot list /dev/input: {e}"
                ))));
                return;
            }
        };
        paths.sort();
        ignored.retain(|p| paths.contains(p));

        for path in paths {
            if ignored.contains(&path) || active.lock().unwrap().contains(&path) {
                continue;
            }
            match Device::open(&path) {
                Ok(dev) if is_keyboard(&dev) || is_mouse(&dev) => {
                    active.lock().unwrap().insert(path.clone());
                    let (sink, active) = (sink.clone(), active.clone());
                    let _ = thread::Builder::new()
                        .name(format!("keyshow-{}", path.display()))
                        .spawn(move || {
                            read_device(dev, &sink);
                            active.lock().unwrap().remove(&path);
                        });
                }
                Ok(_) => {
                    ignored.insert(path);
                }
                Err(e) if e.kind() == ErrorKind::PermissionDenied => denied = true,
                Err(_) => {}
            }
        }

        let n = active.lock().unwrap().len();
        let status = if n == 0 && denied {
            InputStatus::PermissionDenied
        } else if n == 0 {
            InputStatus::Failed("No keyboard or mouse found in /dev/input".into())
        } else {
            InputStatus::Ok(n)
        };
        if last_status.as_ref() != Some(&status) {
            sink.send(InputEvent::Status(status.clone()));
            last_status = Some(status);
        }
        thread::sleep(RESCAN_INTERVAL);
    }
}

fn is_keyboard(dev: &Device) -> bool {
    dev.supported_keys()
        .is_some_and(|k| k.contains(KeyCode::KEY_A) && k.contains(KeyCode::KEY_ENTER))
}

fn is_mouse(dev: &Device) -> bool {
    dev.supported_keys()
        .is_some_and(|k| k.contains(KeyCode::BTN_LEFT))
}

/// Blocks reading one device until it disappears or errors.
fn read_device(mut dev: Device, sink: &Sink) {
    loop {
        let events = match dev.fetch_events() {
            Ok(evs) => evs,
            Err(_) => return,
        };
        for ev in events {
            match ev.destructure() {
                EventSummary::Key(_, code, value) => {
                    if let Some(button) = mouse_button(code) {
                        if value == 1 {
                            sink.send(InputEvent::Mouse(button));
                        }
                    } else if let Some(key) = map_key(code) {
                        sink.send(InputEvent::Key {
                            key,
                            code: u32::from(code.0),
                            pressed: value != 0,
                            repeat: value == 2,
                            text: None,
                        });
                    }
                }
                EventSummary::RelativeAxis(_, axis, _)
                    if axis == RelativeAxisCode::REL_WHEEL
                        || axis == RelativeAxisCode::REL_HWHEEL =>
                {
                    sink.send(InputEvent::Scroll);
                }
                _ => {}
            }
        }
    }
}

fn mouse_button(code: KeyCode) -> Option<MouseButton> {
    Some(match code {
        KeyCode::BTN_LEFT => MouseButton::Left,
        KeyCode::BTN_RIGHT => MouseButton::Right,
        KeyCode::BTN_MIDDLE => MouseButton::Middle,
        KeyCode::BTN_SIDE => MouseButton::Back,
        KeyCode::BTN_EXTRA => MouseButton::Forward,
        _ => return None,
    })
}

/// evdev keycode → [`Key`], assuming a US layout for character keys.
fn map_key(code: KeyCode) -> Option<Key> {
    use Key::*;
    let c = |ch| Some(Char(ch));
    match code {
        KeyCode::KEY_LEFTCTRL | KeyCode::KEY_RIGHTCTRL => Some(Ctrl),
        KeyCode::KEY_LEFTSHIFT | KeyCode::KEY_RIGHTSHIFT => Some(Shift),
        KeyCode::KEY_LEFTALT => Some(Alt),
        KeyCode::KEY_RIGHTALT => Some(AltGr),
        KeyCode::KEY_LEFTMETA | KeyCode::KEY_RIGHTMETA => Some(Super),

        KeyCode::KEY_A => c('a'),
        KeyCode::KEY_B => c('b'),
        KeyCode::KEY_C => c('c'),
        KeyCode::KEY_D => c('d'),
        KeyCode::KEY_E => c('e'),
        KeyCode::KEY_F => c('f'),
        KeyCode::KEY_G => c('g'),
        KeyCode::KEY_H => c('h'),
        KeyCode::KEY_I => c('i'),
        KeyCode::KEY_J => c('j'),
        KeyCode::KEY_K => c('k'),
        KeyCode::KEY_L => c('l'),
        KeyCode::KEY_M => c('m'),
        KeyCode::KEY_N => c('n'),
        KeyCode::KEY_O => c('o'),
        KeyCode::KEY_P => c('p'),
        KeyCode::KEY_Q => c('q'),
        KeyCode::KEY_R => c('r'),
        KeyCode::KEY_S => c('s'),
        KeyCode::KEY_T => c('t'),
        KeyCode::KEY_U => c('u'),
        KeyCode::KEY_V => c('v'),
        KeyCode::KEY_W => c('w'),
        KeyCode::KEY_X => c('x'),
        KeyCode::KEY_Y => c('y'),
        KeyCode::KEY_Z => c('z'),

        KeyCode::KEY_1 | KeyCode::KEY_KP1 => c('1'),
        KeyCode::KEY_2 | KeyCode::KEY_KP2 => c('2'),
        KeyCode::KEY_3 | KeyCode::KEY_KP3 => c('3'),
        KeyCode::KEY_4 | KeyCode::KEY_KP4 => c('4'),
        KeyCode::KEY_5 | KeyCode::KEY_KP5 => c('5'),
        KeyCode::KEY_6 | KeyCode::KEY_KP6 => c('6'),
        KeyCode::KEY_7 | KeyCode::KEY_KP7 => c('7'),
        KeyCode::KEY_8 | KeyCode::KEY_KP8 => c('8'),
        KeyCode::KEY_9 | KeyCode::KEY_KP9 => c('9'),
        KeyCode::KEY_0 | KeyCode::KEY_KP0 => c('0'),

        KeyCode::KEY_MINUS | KeyCode::KEY_KPMINUS => c('-'),
        KeyCode::KEY_EQUAL => c('='),
        KeyCode::KEY_LEFTBRACE => c('['),
        KeyCode::KEY_RIGHTBRACE => c(']'),
        KeyCode::KEY_BACKSLASH => c('\\'),
        KeyCode::KEY_SEMICOLON => c(';'),
        KeyCode::KEY_APOSTROPHE => c('\''),
        KeyCode::KEY_GRAVE => c('`'),
        KeyCode::KEY_COMMA => c(','),
        KeyCode::KEY_DOT | KeyCode::KEY_KPDOT => c('.'),
        KeyCode::KEY_SLASH | KeyCode::KEY_KPSLASH => c('/'),
        KeyCode::KEY_KPPLUS => c('+'),
        KeyCode::KEY_KPASTERISK => c('*'),
        KeyCode::KEY_102ND => c('<'),

        KeyCode::KEY_SPACE => Some(Space),
        KeyCode::KEY_ENTER | KeyCode::KEY_KPENTER => Some(Enter),
        KeyCode::KEY_ESC => Some(Esc),
        KeyCode::KEY_TAB => Some(Tab),
        KeyCode::KEY_BACKSPACE => Some(Backspace),
        KeyCode::KEY_DELETE => Some(Delete),
        KeyCode::KEY_INSERT => Some(Insert),
        KeyCode::KEY_HOME => Some(Home),
        KeyCode::KEY_END => Some(End),
        KeyCode::KEY_PAGEUP => Some(PageUp),
        KeyCode::KEY_PAGEDOWN => Some(PageDown),
        KeyCode::KEY_UP => Some(Up),
        KeyCode::KEY_DOWN => Some(Down),
        KeyCode::KEY_LEFT => Some(Left),
        KeyCode::KEY_RIGHT => Some(Right),
        KeyCode::KEY_CAPSLOCK => Some(CapsLock),
        KeyCode::KEY_NUMLOCK => Some(NumLock),
        KeyCode::KEY_SCROLLLOCK => Some(ScrollLock),
        KeyCode::KEY_SYSRQ => Some(PrintScreen),
        KeyCode::KEY_PAUSE => Some(Pause),
        KeyCode::KEY_COMPOSE | KeyCode::KEY_MENU => Some(Menu),

        KeyCode::KEY_F1 => Some(F(1)),
        KeyCode::KEY_F2 => Some(F(2)),
        KeyCode::KEY_F3 => Some(F(3)),
        KeyCode::KEY_F4 => Some(F(4)),
        KeyCode::KEY_F5 => Some(F(5)),
        KeyCode::KEY_F6 => Some(F(6)),
        KeyCode::KEY_F7 => Some(F(7)),
        KeyCode::KEY_F8 => Some(F(8)),
        KeyCode::KEY_F9 => Some(F(9)),
        KeyCode::KEY_F10 => Some(F(10)),
        KeyCode::KEY_F11 => Some(F(11)),
        KeyCode::KEY_F12 => Some(F(12)),
        // Media / system keys and anything else we don't name are ignored.
        _ => None,
    }
}
