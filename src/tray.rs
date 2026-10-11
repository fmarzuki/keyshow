//! Windows tray icon. (Linux has none: stock GNOME hides tray icons.)

use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::app::AppMsg;
use crate::input::Sink;

pub struct Tray {
    _icon: TrayIcon,
}

pub fn create(sink: Sink) -> Option<Tray> {
    let settings = MenuItem::new("Settings…", true, None);
    let pause = MenuItem::new("Pause / Resume", true, None);
    let quit = MenuItem::new("Quit Keyshow", true, None);
    let menu = Menu::new();
    menu.append_items(&[&settings, &pause, &PredefinedMenuItem::separator(), &quit])
        .ok()?;

    let ids = (settings.id().clone(), pause.id().clone(), quit.id().clone());
    let menu_sink = sink.clone();
    MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
        let msg = if e.id == ids.0 {
            AppMsg::OpenSettings
        } else if e.id == ids.1 {
            AppMsg::TogglePause
        } else if e.id == ids.2 {
            AppMsg::Quit
        } else {
            return;
        };
        menu_sink.send_msg(msg);
    }));
    TrayIconEvent::set_event_handler(Some(move |e: TrayIconEvent| {
        if let TrayIconEvent::DoubleClick {
            button: MouseButton::Left,
            ..
        } = e
        {
            sink.send_msg(AppMsg::OpenSettings);
        }
    }));

    let icon = Icon::from_rgba(crate::icon::rgba(32), 32, 32).ok()?;
    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Keyshow")
        .with_icon(icon)
        .build()
        .ok()?;
    Some(Tray { _icon: tray })
}
