// Release builds on Windows are GUI apps (no console window).
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod config;
mod demo;
mod display;
mod fonts;
mod icon;
mod input;
mod ipc;
mod keys;
mod overlay;
mod settings;
#[cfg(windows)]
mod tray;
#[cfg(target_os = "linux")]
mod x11;

use std::sync::{Arc, OnceLock, mpsc};

use eframe::egui;

const HELP: &str = "\
keyshow — show your keystrokes on screen

USAGE:
    keyshow [OPTIONS]

OPTIONS:
    --settings    Open the Settings window (also what a second launch does)
    --pause       Pause / resume the running instance
    --quit        Quit the running instance
    --demo        Show sample keystrokes instead of capturing real input
    -h, --help    Print this help
    -V, --version Print version

SHORTCUTS:
    Ctrl+Alt+Shift+K   pause / resume
    Ctrl+Alt+Shift+S   open Settings
";

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let has = |flag: &str| args.iter().any(|a| a == flag);
    if has("-h") || has("--help") {
        print!("{HELP}");
        return Ok(());
    }
    if has("-V") || has("--version") {
        println!("keyshow {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if let Some(bad) = args
        .iter()
        .find(|a| !matches!(a.as_str(), "--settings" | "--pause" | "--quit" | "--demo"))
    {
        eprintln!("keyshow: unknown option {bad}\n\n{HELP}");
        std::process::exit(2);
    }

    let command = if has("--quit") {
        "quit"
    } else if has("--pause") {
        "pause"
    } else {
        "settings"
    };
    let listener = match ipc::acquire(command) {
        Ok(ipc::Instance::Primary(l)) => Some(l),
        Ok(ipc::Instance::Forwarded) => {
            println!("keyshow: sent '{command}' to the running instance");
            return Ok(());
        }
        Err(e) => {
            eprintln!("keyshow: single-instance check failed ({e}); continuing");
            None
        }
    };
    if command != "settings" {
        println!("keyshow: not running");
        return Ok(());
    }

    let demo = has("--demo");
    let (cfg, existed) = config::Config::load();
    if !existed {
        // Write defaults so Settings only pops up automatically on the very first run.
        if let Err(e) = cfg.save() {
            eprintln!("keyshow: could not save settings: {e}");
        }
    }
    let (tx, rx) = mpsc::channel();
    let ctx_cell = Arc::new(OnceLock::new());
    let sink = input::Sink::new(tx, ctx_cell.clone());
    if let Some(l) = listener {
        ipc::serve(l, sink.clone());
    }
    if demo {
        demo::spawn(sink.clone(), true);
    } else {
        input::spawn(sink.clone());
    }
    let open_settings = has("--settings") || !existed;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Keyshow Overlay")
            .with_app_id("keyshow")
            .with_inner_size([800.0, 200.0])
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_mouse_passthrough(true)
            .with_active(false)
            .with_taskbar(false)
            .with_resizable(false)
            .with_has_shadow(false)
            .with_window_type(egui::X11WindowType::Notification)
            .with_icon(icon::icon_data(64)),
        event_loop_builder: Some(Box::new(|_builder| {
            // Wayland compositors don't let clients position windows or keep them on
            // top, so on Linux the overlay runs through XWayland when available.
            #[cfg(target_os = "linux")]
            if std::env::var_os("DISPLAY").is_some() {
                use winit::platform::x11::EventLoopBuilderExtX11;
                _builder.with_x11();
            }
        })),
        ..Default::default()
    };

    eframe::run_native(
        "keyshow",
        options,
        Box::new(move |cc| {
            let _ = ctx_cell.set(cc.egui_ctx.clone());
            fonts::install(&cc.egui_ctx);
            Ok(Box::new(app::KeyshowApp::new(
                cfg,
                rx,
                sink,
                open_settings,
                demo,
            )))
        }),
    )
}
