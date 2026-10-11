//! Windows backend: low-level keyboard and mouse hooks on a dedicated thread.
//! Hooks only observe; every event is passed on with `CallNextHookEx`.

use std::collections::HashSet;
use std::ptr::null_mut;
use std::sync::{Mutex, OnceLock};
use std::thread;

use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetForegroundWindow, GetMessageW, GetWindowThreadProcessId,
    HC_ACTION, KBDLLHOOKSTRUCT, MSG, MSLLHOOKSTRUCT, SetWindowsHookExW, TranslateMessage,
    WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_MBUTTONDOWN,
    WM_MOUSEHWHEEL, WM_MOUSEWHEEL, WM_RBUTTONDOWN, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_XBUTTONDOWN,
};

use super::{InputEvent, InputStatus, MouseButton, Sink};
use crate::keys::Key;

static SINK: OnceLock<Sink> = OnceLock::new();
/// Virtual-key codes currently held, to detect auto-repeat and build key state.
static HELD: Mutex<Option<HashSet<u32>>> = Mutex::new(None);

/// `ToUnicodeEx` flag: don't change kernel keyboard state (keeps dead keys working).
const TOUNICODE_NO_STATE_CHANGE: u32 = 0x4;
/// Scan code of the fake LCtrl that Windows sends along with AltGr.
const ALTGR_FAKE_CTRL_SCAN: u32 = 0x21D;

pub fn spawn(sink: Sink) {
    let _ = SINK.set(sink);
    thread::Builder::new()
        .name("keyshow-hooks".into())
        .spawn(|| unsafe { hook_thread() })
        .expect("spawn hook thread");
}

unsafe fn hook_thread() {
    let sink = SINK.get().expect("sink set");
    let module = unsafe { GetModuleHandleW(std::ptr::null()) };
    let kb = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), module, 0) };
    let mouse = unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), module, 0) };
    if kb.is_null() || mouse.is_null() {
        sink.send(InputEvent::Status(InputStatus::Failed(
            "SetWindowsHookEx failed".into(),
        )));
        return;
    }
    sink.send(InputEvent::Status(InputStatus::Ok(2)));

    // Low-level hooks are delivered through this thread's message loop.
    let mut msg: MSG = unsafe { std::mem::zeroed() };
    while unsafe { GetMessageW(&mut msg, null_mut(), 0, 0) } > 0 {
        unsafe {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let info = unsafe { &*(lparam as *const KBDLLHOOKSTRUCT) };
        let pressed = matches!(wparam as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
        let released = matches!(wparam as u32, WM_KEYUP | WM_SYSKEYUP);
        if (pressed || released) && info.scanCode != ALTGR_FAKE_CTRL_SCAN {
            handle_key(info.vkCode, info.scanCode, pressed);
        }
    }
    unsafe { CallNextHookEx(null_mut(), code, wparam, lparam) }
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32
        && let Some(sink) = SINK.get()
    {
        let info = unsafe { &*(lparam as *const MSLLHOOKSTRUCT) };
        let ev = match wparam as u32 {
            WM_LBUTTONDOWN => Some(InputEvent::Mouse(MouseButton::Left)),
            WM_RBUTTONDOWN => Some(InputEvent::Mouse(MouseButton::Right)),
            WM_MBUTTONDOWN => Some(InputEvent::Mouse(MouseButton::Middle)),
            WM_XBUTTONDOWN => Some(InputEvent::Mouse(if (info.mouseData >> 16) == 1 {
                MouseButton::Back
            } else {
                MouseButton::Forward
            })),
            WM_MOUSEWHEEL | WM_MOUSEHWHEEL => Some(InputEvent::Scroll),
            _ => None,
        };
        if let Some(ev) = ev {
            sink.send(ev);
        }
    }
    unsafe { CallNextHookEx(null_mut(), code, wparam, lparam) }
}

fn handle_key(vk: u32, scan: u32, pressed: bool) {
    let Some(sink) = SINK.get() else { return };
    let mut guard = HELD.lock().unwrap();
    let held = guard.get_or_insert_with(HashSet::new);
    let repeat = pressed && !held.insert(vk);
    if !pressed {
        held.remove(&vk);
    }

    let layout = unsafe {
        let fg = GetForegroundWindow();
        GetKeyboardLayout(GetWindowThreadProcessId(fg, null_mut()))
    };
    let Some(key) = map_vk(vk, layout) else {
        return;
    };
    let text = if pressed && matches!(key, Key::Char(_)) {
        typed_text(vk, scan, held, layout)
    } else {
        None
    };
    drop(guard);
    sink.send(InputEvent::Key {
        key,
        code: vk,
        pressed,
        repeat,
        text,
    });
}

/// Text produced by `vk` given the modifiers we have seen held, in the foreground layout.
fn typed_text(vk: u32, scan: u32, held: &HashSet<u32>, layout: HKL) -> Option<String> {
    let down = |codes: &[VIRTUAL_KEY]| codes.iter().any(|c| held.contains(&u32::from(*c)));
    let mut state = [0u8; 256];
    if down(&[VK_LSHIFT, VK_RSHIFT, VK_SHIFT]) {
        state[VK_SHIFT as usize] = 0x80;
    }
    if down(&[VK_RMENU]) {
        // AltGr == Ctrl+Alt for character translation.
        state[VK_CONTROL as usize] = 0x80;
        state[VK_MENU as usize] = 0x80;
    }
    if unsafe { GetKeyState(i32::from(VK_CAPITAL)) } & 1 != 0 {
        state[VK_CAPITAL as usize] = 1;
    }
    let mut buf = [0u16; 8];
    let n = unsafe {
        ToUnicodeEx(
            vk,
            scan,
            state.as_ptr(),
            buf.as_mut_ptr(),
            buf.len() as i32,
            TOUNICODE_NO_STATE_CHANGE,
            layout,
        )
    };
    // n < 0: dead key; 0: no translation.
    if n <= 0 {
        return None;
    }
    let s = String::from_utf16_lossy(&buf[..n as usize]);
    (!s.chars().any(char::is_control)).then_some(s)
}

fn map_vk(vk: u32, layout: HKL) -> Option<Key> {
    use Key::*;
    let Ok(v) = VIRTUAL_KEY::try_from(vk) else {
        return None;
    };
    Some(match v {
        VK_CONTROL | VK_LCONTROL | VK_RCONTROL => Ctrl,
        VK_SHIFT | VK_LSHIFT | VK_RSHIFT => Shift,
        VK_MENU | VK_LMENU => Alt,
        VK_RMENU => AltGr,
        VK_LWIN | VK_RWIN => Super,
        VK_SPACE => Space,
        VK_RETURN => Enter,
        VK_ESCAPE => Esc,
        VK_TAB => Tab,
        VK_BACK => Backspace,
        VK_DELETE => Delete,
        VK_INSERT => Insert,
        VK_HOME => Home,
        VK_END => End,
        VK_PRIOR => PageUp,
        VK_NEXT => PageDown,
        VK_UP => Up,
        VK_DOWN => Down,
        VK_LEFT => Left,
        VK_RIGHT => Right,
        VK_CAPITAL => CapsLock,
        VK_NUMLOCK => NumLock,
        VK_SCROLL => ScrollLock,
        VK_SNAPSHOT => PrintScreen,
        VK_PAUSE => Pause,
        VK_APPS => Menu,
        VK_F1..=VK_F24 => F((v - VK_F1 + 1) as u8),
        VK_NUMPAD0..=VK_NUMPAD9 => Char(char::from(b'0' + (v - VK_NUMPAD0) as u8)),
        VK_MULTIPLY => Char('*'),
        VK_ADD => Char('+'),
        VK_SUBTRACT => Char('-'),
        VK_DECIMAL => Char('.'),
        VK_DIVIDE => Char('/'),
        // 'A'..'Z' and '0'..'9' share their ASCII codes.
        0x30..=0x39 | 0x41..=0x5A => Char(char::from(v as u8).to_ascii_lowercase()),
        _ => {
            // OEM punctuation: ask the layout which base character it produces.
            let ch = unsafe { MapVirtualKeyExW(vk, MAPVK_VK_TO_CHAR, layout) } & 0x7FFF;
            match char::from_u32(ch) {
                Some(c) if ch != 0 && !c.is_control() => Char(c.to_lowercase().next().unwrap_or(c)),
                _ => return None,
            }
        }
    })
}
