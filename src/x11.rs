//! X11-only window tweaks winit doesn't expose.

use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{ClientMessageEvent, ConnectionExt, EventMask};

/// `_NET_WM_STATE` action: add the property.
const NET_WM_STATE_ADD: u32 = 1;
/// `_NET_WM_DESKTOP` value meaning "all workspaces".
const ALL_DESKTOPS: u32 = 0xFFFF_FFFF;

/// Shows the overlay on every workspace, so it doesn't vanish when switching.
pub fn make_sticky(frame: &eframe::Frame) {
    let Some(window) = frame.winit_window() else {
        return;
    };
    let Ok(handle) = window.window_handle() else {
        return;
    };
    let win = match handle.as_raw() {
        RawWindowHandle::Xlib(h) => h.window as u32,
        RawWindowHandle::Xcb(h) => h.window.get(),
        _ => return, // Not running on X11.
    };
    if let Err(e) = send_sticky(win) {
        eprintln!("keyshow: could not make overlay sticky: {e}");
    }
}

fn send_sticky(win: u32) -> Result<(), Box<dyn std::error::Error>> {
    let (conn, screen) = x11rb::connect(None)?;
    let root = conn.setup().roots[screen].root;
    let atom = |name: &[u8]| -> Result<u32, Box<dyn std::error::Error>> {
        Ok(conn.intern_atom(false, name)?.reply()?.atom)
    };
    let (desktop, state, sticky) = (
        atom(b"_NET_WM_DESKTOP")?,
        atom(b"_NET_WM_STATE")?,
        atom(b"_NET_WM_STATE_STICKY")?,
    );
    let mask = EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY;
    // Source indication 1 = normal application.
    for (kind, data) in [
        (desktop, [ALL_DESKTOPS, 1, 0, 0, 0]),
        (state, [NET_WM_STATE_ADD, sticky, 0, 1, 0]),
    ] {
        conn.send_event(
            false,
            root,
            mask,
            ClientMessageEvent::new(32, win, kind, data),
        )?
        // Round-trip so the requests are processed before the connection closes.
        .check()?;
    }
    Ok(())
}
