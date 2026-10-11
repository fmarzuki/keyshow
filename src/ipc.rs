//! Single-instance handling over a local socket (abstract socket on Linux,
//! named pipe on Windows). A second launch forwards a command and exits.

use std::io::{self, BufRead, BufReader, Write};
use std::thread;

use interprocess::local_socket::{
    GenericNamespaced, Listener, ListenerOptions, Stream, prelude::*,
};

use crate::app::AppMsg;
use crate::input::Sink;

fn socket_name() -> String {
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_default();
    format!("keyshow-{user}.sock")
}

pub enum Instance {
    /// We are the running instance; serve commands from later launches.
    Primary(Listener),
    /// Another instance is running and received `command`.
    Forwarded,
}

/// Becomes the primary instance, or forwards `command` ("settings" / "quit") to it.
pub fn acquire(command: &str) -> io::Result<Instance> {
    let name = socket_name();
    match ListenerOptions::new()
        .name(name.as_str().to_ns_name::<GenericNamespaced>()?)
        .create_sync()
    {
        Ok(listener) => Ok(Instance::Primary(listener)),
        Err(e) if e.kind() == io::ErrorKind::AddrInUse => {
            let mut stream = Stream::connect(name.as_str().to_ns_name::<GenericNamespaced>()?)?;
            stream.write_all(format!("{command}\n").as_bytes())?;
            Ok(Instance::Forwarded)
        }
        Err(e) => Err(e),
    }
}

pub fn serve(listener: Listener, sink: Sink) {
    let _ = thread::Builder::new()
        .name("keyshow-ipc".into())
        .spawn(move || {
            for conn in listener.incoming().filter_map(Result::ok) {
                let mut line = String::new();
                if BufReader::new(conn).read_line(&mut line).is_err() {
                    continue;
                }
                match line.trim() {
                    "settings" => sink.send_msg(AppMsg::OpenSettings),
                    "pause" => sink.send_msg(AppMsg::TogglePause),
                    "quit" => sink.send_msg(AppMsg::Quit),
                    _ => {}
                }
            }
        });
}
