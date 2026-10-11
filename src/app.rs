//! The eframe app: owns the overlay (root viewport) and the Settings window.

use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use eframe::egui::{self, Rect, ViewportCommand, ViewportId, pos2, vec2};

use crate::config::Config;
use crate::display::{Action, Engine};
use crate::input::{InputEvent, InputStatus, Sink};
use crate::settings::{self, SettingsAction};
use crate::{demo, icon, overlay};

pub enum AppMsg {
    Input(InputEvent),
    OpenSettings,
    TogglePause,
    Quit,
}

pub struct Monitor {
    pub name: String,
    /// Physical pixels.
    pos: (f32, f32),
    size: (f32, f32),
    scale: f32,
}

/// How often the monitor list is refreshed (hotplug, resolution changes).
const MONITOR_REFRESH: Duration = Duration::from_secs(5);
/// Frames to wait after (re)positioning before re-asserting always-on-top.
/// Window managers ignore "above" requests sent before the window is mapped.
const RAISE_DELAY_FRAMES: u32 = 3;

pub struct KeyshowApp {
    cfg: Config,
    saved: Config,
    engine: Engine,
    rx: Receiver<AppMsg>,
    sink: Sink,
    settings_open: bool,
    status: Option<InputStatus>,
    demo: bool,
    monitors: Vec<Monitor>,
    monitors_at: Option<Instant>,
    applied: Option<Rect>,
    raise_in: u32,
    #[cfg(windows)]
    _tray: Option<crate::tray::Tray>,
}

impl KeyshowApp {
    pub fn new(
        cfg: Config,
        rx: Receiver<AppMsg>,
        sink: Sink,
        open_settings: bool,
        demo: bool,
    ) -> Self {
        Self {
            saved: cfg.clone(),
            cfg,
            engine: Engine::default(),
            rx,
            #[cfg(windows)]
            _tray: crate::tray::create(sink.clone()),
            sink,
            settings_open: open_settings,
            status: None,
            demo,
            monitors: Vec::new(),
            monitors_at: None,
            applied: None,
            raise_in: RAISE_DELAY_FRAMES,
        }
    }

    fn open_settings(&mut self, ctx: &egui::Context) {
        if self.settings_open {
            ctx.send_viewport_cmd_to(settings_id(), ViewportCommand::Visible(true));
            ctx.send_viewport_cmd_to(settings_id(), ViewportCommand::Focus);
        }
        self.settings_open = true;
    }

    fn drain_messages(&mut self, ctx: &egui::Context, now: Instant) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                AppMsg::Input(InputEvent::Status(s)) => {
                    if s == InputStatus::PermissionDenied {
                        self.settings_open = true;
                    }
                    self.status = Some(s);
                }
                AppMsg::Input(ev) => match self.engine.handle(&ev, &self.cfg, now) {
                    Action::OpenSettings => self.open_settings(ctx),
                    Action::TogglePause | Action::None => {}
                },
                AppMsg::OpenSettings => self.open_settings(ctx),
                AppMsg::TogglePause => self.engine.toggle_pause(&self.cfg, now),
                AppMsg::Quit => ctx.send_viewport_cmd(ViewportCommand::Close),
            }
        }
    }

    fn refresh_monitors(&mut self, frame: &eframe::Frame, now: Instant) {
        if self
            .monitors_at
            .is_some_and(|t| now.duration_since(t) < MONITOR_REFRESH)
        {
            return;
        }
        self.monitors_at = Some(now);
        let Some(window) = frame.winit_window() else {
            return;
        };
        self.monitors = window
            .available_monitors()
            .enumerate()
            .map(|(i, m)| {
                let (p, s) = (m.position(), m.size());
                Monitor {
                    name: format!(
                        "{}: {} ({}×{})",
                        i + 1,
                        m.name().unwrap_or_else(|| "Display".into()),
                        s.width,
                        s.height
                    ),
                    pos: (p.x as f32, p.y as f32),
                    size: (s.width as f32, s.height as f32),
                    scale: m.scale_factor() as f32,
                }
            })
            .collect();
    }

    /// Keeps the overlay window covering a full-width band at the anchored edge.
    fn place_overlay(&mut self, ctx: &egui::Context) {
        let Some(mon) = self
            .monitors
            .get(self.cfg.monitor)
            .or(self.monitors.first())
        else {
            return;
        };
        let ppp = ctx.pixels_per_point();
        // Position is converted with the window's current scale so it lands on the
        // right physical pixel; size uses the target monitor's own scale.
        let origin = pos2(mon.pos.0 / ppp, mon.pos.1 / ppp);
        let size = vec2(mon.size.0 / mon.scale, mon.size.1 / mon.scale);
        let band = overlay::band_height(&self.cfg).min(size.y);
        let y = match self.cfg.anchor.align().1 {
            0 => self.cfg.offset_y,
            1 => (size.y - band) / 2.0,
            _ => size.y - band - self.cfg.offset_y,
        }
        .clamp(0.0, size.y - band);
        let target = Rect::from_min_size(origin + vec2(0.0, y), vec2(size.x, band));

        if self.applied != Some(target) {
            ctx.send_viewport_cmd(ViewportCommand::OuterPosition(target.min));
            ctx.send_viewport_cmd(ViewportCommand::InnerSize(target.size()));
            self.applied = Some(target);
            self.raise_in = RAISE_DELAY_FRAMES;
        }
    }

    fn settings_window(&mut self, ctx: &egui::Context) {
        let builder = egui::ViewportBuilder::default()
            .with_title("Keyshow Settings")
            .with_app_id("keyshow")
            .with_inner_size([500.0, 720.0])
            .with_min_inner_size([420.0, 400.0])
            .with_icon(icon::icon_data(64));

        let monitor_names: Vec<String> = self.monitors.iter().map(|m| m.name.clone()).collect();
        let mut action = SettingsAction::None;
        let mut close = false;
        ctx.show_viewport_immediate(settings_id(), builder, |ui, _class| {
            action = settings::show(
                ui,
                &mut settings::View {
                    cfg: &mut self.cfg,
                    status: self.status.as_ref(),
                    paused: &mut self.engine.paused,
                    monitors: &monitor_names,
                    demo: self.demo,
                },
            );
            close = ui.input(|i| i.viewport().close_requested());
        });
        if close {
            self.settings_open = false;
        }
        match action {
            SettingsAction::None => {}
            SettingsAction::Test => demo::spawn(self.sink.clone(), false),
            SettingsAction::Quit => ctx.send_viewport_cmd(ViewportCommand::Close),
        }
    }
}

fn settings_id() -> ViewportId {
    ViewportId::from_hash_of("keyshow-settings")
}

impl eframe::App for KeyshowApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }

    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let now = Instant::now();
        self.drain_messages(ctx, now);
        self.engine.prune(&self.cfg, now);
        self.refresh_monitors(frame, now);
        self.place_overlay(ctx);

        if self.raise_in > 0 {
            self.raise_in -= 1;
            if self.raise_in == 0 {
                ctx.send_viewport_cmd(ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
                #[cfg(target_os = "linux")]
                crate::x11::make_sticky(frame);
            }
            ctx.request_repaint();
        }

        if self.cfg != self.saved {
            if let Err(e) = self.cfg.save() {
                eprintln!("keyshow: could not save settings: {e}");
            }
            self.saved = self.cfg.clone();
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        let animating = overlay::paint(ui, &self.engine, &self.cfg, now);
        if animating {
            ui.ctx().request_repaint();
        } else if let Some(next) = self
            .engine
            .bubbles()
            .map(|b| {
                (b.updated + Duration::from_millis(self.cfg.duration_ms))
                    .saturating_duration_since(now)
            })
            .min()
        {
            // Sleep until the next bubble starts fading.
            ui.ctx().request_repaint_after(next);
        }

        if self.settings_open {
            let ctx = ui.ctx().clone();
            self.settings_window(&ctx);
        }
    }
}
