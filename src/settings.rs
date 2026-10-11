//! The Settings window. Every change applies live and is saved automatically.

use eframe::egui::{self, Color32, RichText, Ui};

use crate::config::{Anchor, Config, KeyStyle, Mode, Theme, ThemeName};
use crate::input::InputStatus;
use crate::keys::ModifierStyle;

pub struct View<'a> {
    pub cfg: &'a mut Config,
    pub status: Option<&'a InputStatus>,
    pub paused: &'a mut bool,
    pub monitors: &'a [String],
    pub demo: bool,
}

pub enum SettingsAction {
    None,
    Test,
    Quit,
}

pub fn show(ui: &mut Ui, v: &mut View) -> SettingsAction {
    let mut action = SettingsAction::None;
    egui::CentralPanel::default().show(ui, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            header(ui, v, &mut action);
            ui.separator();
            behaviour(ui, v.cfg);
            ui.separator();
            appearance(ui, v.cfg);
            ui.separator();
            position(ui, v);
            ui.separator();
            timing(ui, v.cfg);
            ui.separator();
            hotkeys(ui);
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Reset to defaults").clicked() {
                    *v.cfg = Config::default();
                }
                if ui.button("Quit Keyshow").clicked() {
                    action = SettingsAction::Quit;
                }
            });
        });
    });
    action
}

fn section(ui: &mut Ui, title: &str) {
    ui.add_space(4.0);
    ui.label(RichText::new(title).strong().size(16.0));
}

fn header(ui: &mut Ui, v: &mut View, action: &mut SettingsAction) {
    ui.horizontal(|ui| {
        ui.heading("⌨ Keyshow");
        ui.label(RichText::new(env!("CARGO_PKG_VERSION")).weak());
    });
    ui.label("Shows your keystrokes on screen for tutorials, demos and recordings.");
    ui.add_space(6.0);

    let (text, color) = match (v.demo, v.status) {
        (true, _) => (
            "Demo mode: showing sample keystrokes".to_owned(),
            Color32::LIGHT_BLUE,
        ),
        (_, Some(InputStatus::Ok(n))) => (
            format!("● Capturing input ({n} devices)"),
            Color32::from_rgb(34, 197, 94),
        ),
        (_, Some(InputStatus::PermissionDenied)) => (
            "● No permission to read keyboard".to_owned(),
            Color32::from_rgb(239, 68, 68),
        ),
        (_, Some(InputStatus::Failed(e))) => (
            format!("● Input error: {e}"),
            Color32::from_rgb(239, 68, 68),
        ),
        (_, None) => ("● Starting…".to_owned(), Color32::GRAY),
    };
    ui.label(RichText::new(text).color(color));

    if v.status == Some(&InputStatus::PermissionDenied) {
        permission_help(ui);
    }

    ui.horizontal(|ui| {
        let label = if *v.paused { "▶ Resume" } else { "⏸ Pause" };
        if ui.button(label).clicked() {
            *v.paused = !*v.paused;
        }
        if ui.button("Show test keystrokes").clicked() {
            *action = SettingsAction::Test;
        }
    });
}

fn permission_help(ui: &mut Ui) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.label(
            "On Linux, Keyshow reads keyboards from /dev/input, which needs permission. \
             Run one of these in a terminal (Keyshow picks up the new access by itself):",
        );
        ui.add_space(4.0);
        ui.label(RichText::new("Recommended: let the logged-in user read keyboards and mice").strong());
        copyable(ui, "packaging/linux/install.sh");
        ui.label("or, manually:");
        copyable(
            ui,
            "sudo cp packaging/linux/70-keyshow.rules /etc/udev/rules.d/ && \
             sudo udevadm control --reload && \
             sudo udevadm trigger --subsystem-match=input --action=change",
        );
        ui.label(RichText::new("Alternative: join the input group (log out and back in afterwards)").strong());
        copyable(ui, "sudo usermod -aG input $USER");
        ui.label(
            RichText::new("Either way, programs you run can read every keystroke. Keyshow never stores or sends them.")
                .weak(),
        );
    });
}

fn copyable(ui: &mut Ui, cmd: &str) {
    // Button first, so the wrapped command gets the remaining width.
    ui.horizontal_top(|ui| {
        if ui.small_button("Copy").clicked() {
            ui.ctx().copy_text(cmd.to_owned());
        }
        ui.add(egui::Label::new(RichText::new(cmd).monospace()).wrap());
    });
}

fn behaviour(ui: &mut Ui, cfg: &mut Config) {
    section(ui, "What to show");
    ui.radio_value(
        &mut cfg.mode,
        Mode::CombosOnly,
        "Shortcuts only (Ctrl / Alt / Super + key)",
    );
    ui.radio_value(
        &mut cfg.mode,
        Mode::AllKeys,
        "All keys (typing is shown as text)",
    );
    ui.add_space(4.0);
    ui.checkbox(&mut cfg.show_function_keys, "Function keys (F1–F12)");
    ui.checkbox(
        &mut cfg.show_special_keys,
        "Special keys (Enter, Esc, Tab, arrows, …)",
    );
    ui.checkbox(
        &mut cfg.show_lone_modifiers,
        "Modifiers pressed on their own",
    );
    ui.checkbox(&mut cfg.show_mouse, "Mouse clicks and scrolling");
}

fn appearance(ui: &mut Ui, cfg: &mut Config) {
    section(ui, "Appearance");
    egui::Grid::new("appearance")
        .num_columns(2)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            ui.label("Theme");
            ui.horizontal_wrapped(|ui| {
                for t in ThemeName::ALL {
                    ui.selectable_value(&mut cfg.theme, t, t.name());
                }
            });
            ui.end_row();

            ui.label("Key style");
            ui.horizontal(|ui| {
                ui.selectable_value(&mut cfg.key_style, KeyStyle::Pill, "Pill");
                ui.selectable_value(&mut cfg.key_style, KeyStyle::Keycaps, "Keycaps");
            });
            ui.end_row();

            ui.label("Key labels");
            ui.horizontal(|ui| {
                ui.selectable_value(&mut cfg.modifier_style, ModifierStyle::Text, "Ctrl Shift");
                ui.selectable_value(&mut cfg.modifier_style, ModifierStyle::Symbols, "⌃ ⇧ ⌥ ⏎");
            });
            ui.end_row();

            ui.label("Font size");
            ui.add(egui::Slider::new(&mut cfg.font_size, 12.0..=96.0).suffix(" pt"));
            ui.end_row();

            ui.label("Opacity");
            ui.add(egui::Slider::new(&mut cfg.opacity, 0.1..=1.0).fixed_decimals(2));
            ui.end_row();
        });

    if cfg.theme == ThemeName::Custom {
        ui.add_space(6.0);
        custom_theme(ui, &mut cfg.custom_theme);
    }
}

fn custom_theme(ui: &mut Ui, t: &mut Theme) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.label(RichText::new("Custom theme").strong());
        egui::Grid::new("custom-theme")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                for (label, c) in [
                    ("Text", &mut t.text),
                    ("Background", &mut t.background),
                    ("Border", &mut t.border),
                    ("Counter (×N)", &mut t.accent),
                ] {
                    ui.label(label);
                    ui.color_edit_button_srgba_unmultiplied(c);
                    ui.end_row();
                }
                ui.label("Border width");
                ui.add(egui::Slider::new(&mut t.border_width, 0.0..=6.0));
                ui.end_row();
                ui.label("Corner radius");
                ui.add(egui::Slider::new(&mut t.corner_radius, 0.0..=40.0));
                ui.end_row();
                ui.label("Shadow");
                ui.checkbox(&mut t.shadow, "");
                ui.end_row();
            });
        ui.horizontal(|ui| {
            ui.label("Start from:");
            for name in &ThemeName::ALL[..5] {
                if ui.small_button(name.name()).clicked() {
                    *t = Theme::builtin(*name);
                }
            }
        });
    });
}

fn position(ui: &mut Ui, v: &mut View) {
    let cfg = &mut *v.cfg;
    section(ui, "Position");
    ui.horizontal(|ui| {
        egui::Grid::new("anchor")
            .spacing([4.0, 4.0])
            .show(ui, |ui| {
                for (i, a) in Anchor::ALL.iter().enumerate() {
                    let selected = cfg.anchor == *a;
                    let dot = if selected { "●" } else { "○" };
                    if ui
                        .add_sized([34.0, 26.0], egui::Button::selectable(selected, dot))
                        .clicked()
                    {
                        cfg.anchor = *a;
                    }
                    if i % 3 == 2 {
                        ui.end_row();
                    }
                }
            });
        ui.add_space(12.0);
        egui::Grid::new("offsets")
            .num_columns(2)
            .spacing([8.0, 8.0])
            .show(ui, |ui| {
                ui.label("Display");
                let current = v
                    .monitors
                    .get(cfg.monitor)
                    .cloned()
                    .unwrap_or_else(|| format!("{}: (not connected)", cfg.monitor + 1));
                egui::ComboBox::from_id_salt("monitor")
                    .selected_text(current)
                    .width(200.0)
                    .show_ui(ui, |ui| {
                        for (i, name) in v.monitors.iter().enumerate() {
                            ui.selectable_value(&mut cfg.monitor, i, name);
                        }
                    });
                ui.end_row();
                ui.label("Side margin");
                ui.add(egui::Slider::new(&mut cfg.offset_x, 0.0..=600.0).suffix(" pt"));
                ui.end_row();
                ui.label("Edge margin");
                ui.add(egui::Slider::new(&mut cfg.offset_y, 0.0..=600.0).suffix(" pt"));
                ui.end_row();
            });
    });
}

fn timing(ui: &mut Ui, cfg: &mut Config) {
    section(ui, "Timing");
    egui::Grid::new("timing")
        .num_columns(2)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            ui.label("Show for");
            ui.add(
                egui::Slider::new(&mut cfg.duration_ms, 300..=10_000)
                    .suffix(" ms")
                    .logarithmic(true),
            );
            ui.end_row();
            ui.label("Fade out");
            ui.add(egui::Slider::new(&mut cfg.fade_ms, 0..=2_000).suffix(" ms"));
            ui.end_row();
            ui.label("History");
            ui.add(egui::Slider::new(&mut cfg.max_bubbles, 1..=8).suffix(" bubbles"));
            ui.end_row();
        });
}

fn hotkeys(ui: &mut Ui) {
    section(ui, "Global shortcuts");
    egui::Grid::new("hotkeys")
        .num_columns(2)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            ui.label(RichText::new("Ctrl + Alt + Shift + K").monospace());
            ui.label("Pause / resume (use before typing passwords)");
            ui.end_row();
            ui.label(RichText::new("Ctrl + Alt + Shift + S").monospace());
            ui.label("Open this window");
            ui.end_row();
        });
    ui.label(RichText::new("Launching Keyshow again while it runs also opens this window.").weak());
}
