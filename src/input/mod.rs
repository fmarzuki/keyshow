//! Global keyboard and mouse capture. Each platform backend pushes
//! [`InputEvent`]s into a [`Sink`]; nothing is ever written to disk.

use std::sync::{Arc, OnceLock, mpsc::Sender};

use crate::keys::Key;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(windows)]
mod windows;

#[derive(Clone, Debug, PartialEq)]
pub enum InputEvent {
    Key {
        key: Key,
        /// Platform code of the physical key, used to tell left/right modifiers apart.
        code: u32,
        pressed: bool,
        repeat: bool,
        /// Text the key produces with the current modifiers, when the backend knows it
        /// (layout-aware). `None` means "derive it from `key`".
        text: Option<String>,
    },
    Mouse(MouseButton),
    Scroll,
    Status(InputStatus),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
}

impl MouseButton {
    pub fn label(self) -> &'static str {
        match self {
            MouseButton::Left => "Left Click",
            MouseButton::Right => "Right Click",
            MouseButton::Middle => "Middle Click",
            MouseButton::Back => "Mouse Back",
            MouseButton::Forward => "Mouse Forward",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputStatus {
    /// Capturing from this many devices.
    Ok(usize),
    /// Devices exist but we may not read them (Linux: missing udev rule / input group).
    PermissionDenied,
    Failed(String),
}

#[derive(Clone)]
pub struct Sink {
    tx: Sender<crate::app::AppMsg>,
    ctx: Arc<OnceLock<eframe::egui::Context>>,
}

impl Sink {
    pub fn new(tx: Sender<crate::app::AppMsg>, ctx: Arc<OnceLock<eframe::egui::Context>>) -> Self {
        Self { tx, ctx }
    }

    pub fn send(&self, ev: InputEvent) {
        self.send_msg(crate::app::AppMsg::Input(ev));
    }

    pub fn send_msg(&self, msg: crate::app::AppMsg) {
        if self.tx.send(msg).is_ok()
            && let Some(ctx) = self.ctx.get()
        {
            ctx.request_repaint();
        }
    }
}

/// Start capturing global input on a background thread.
pub fn spawn(sink: Sink) {
    #[cfg(target_os = "linux")]
    linux::spawn(sink);
    #[cfg(windows)]
    windows::spawn(sink);
    #[cfg(not(any(target_os = "linux", windows)))]
    sink.send(InputEvent::Status(InputStatus::Failed(
        "This platform is not supported".into(),
    )));
}
