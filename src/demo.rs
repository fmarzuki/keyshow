//! Synthetic keystrokes, for trying out the look without real input capture.

use std::thread;
use std::time::Duration;

use crate::input::{InputEvent, InputStatus, MouseButton, Sink};
use crate::keys::Key;

enum Step {
    Combo(&'static [Key], Key),
    Type(&'static str),
    Click(MouseButton),
    Wait(u64),
}

const SCRIPT: &[Step] = &[
    Step::Combo(&[Key::Ctrl], Key::Char('c')),
    Step::Wait(500),
    Step::Combo(&[Key::Ctrl], Key::Char('v')),
    Step::Wait(900),
    Step::Combo(&[Key::Ctrl, Key::Shift], Key::Char('t')),
    Step::Wait(900),
    Step::Type("Hello, Keyshow!"),
    Step::Combo(&[], Key::Enter),
    Step::Wait(900),
    Step::Combo(&[Key::Ctrl], Key::Char('z')),
    Step::Combo(&[Key::Ctrl], Key::Char('z')),
    Step::Combo(&[Key::Ctrl], Key::Char('z')),
    Step::Wait(900),
    Step::Click(MouseButton::Left),
    Step::Wait(150),
    Step::Click(MouseButton::Left),
    Step::Wait(900),
    Step::Combo(&[Key::Alt], Key::Tab),
    Step::Wait(900),
    Step::Combo(&[], Key::F(5)),
    Step::Wait(900),
    Step::Combo(&[Key::Super], Key::Char('l')),
    Step::Wait(2500),
];

/// Plays the script once, or forever with `looping`.
pub fn spawn(sink: Sink, looping: bool) {
    let _ = thread::Builder::new()
        .name("keyshow-demo".into())
        .spawn(move || {
            if looping {
                sink.send(InputEvent::Status(InputStatus::Ok(0)));
            }
            loop {
                for step in SCRIPT {
                    play(&sink, step);
                }
                if !looping {
                    break;
                }
            }
        });
}

fn key(sink: &Sink, key: Key, pressed: bool, text: Option<String>) {
    // Distinct fake codes per key so modifier tracking behaves like real input.
    let code = 10_000 + format!("{key:?}").bytes().map(u32::from).sum::<u32>();
    sink.send(InputEvent::Key {
        key,
        code,
        pressed,
        repeat: false,
        text,
    });
}

fn play(sink: &Sink, step: &Step) {
    let pause = |ms| thread::sleep(Duration::from_millis(ms));
    match step {
        Step::Combo(mods, k) => {
            for m in *mods {
                key(sink, *m, true, None);
            }
            key(sink, *k, true, None);
            key(sink, *k, false, None);
            for m in mods.iter().rev() {
                key(sink, *m, false, None);
            }
            pause(120);
        }
        Step::Type(s) => {
            for c in s.chars() {
                let k = if c == ' ' {
                    Key::Space
                } else {
                    Key::Char(c.to_ascii_lowercase())
                };
                key(sink, k, true, Some(c.to_string()));
                key(sink, k, false, None);
                pause(70);
            }
        }
        Step::Click(b) => sink.send(InputEvent::Mouse(*b)),
        Step::Wait(ms) => pause(*ms),
    }
}
