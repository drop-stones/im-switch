//! IPC server mode: a loopback daemon that executes `im-switch` commands
//! received over TCP, so consumers can avoid the per-call cost of spawning a
//! fresh process for every IME switch (notably a Windows `.exe` from WSL2).
//!
//! See `docs/wsl2-ipc-design.md` for the full design.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

use im_switch::{get_input_method, list_input_methods, set_input_method, ImSwitchError};

/// Default TCP port for the IPC daemon.
pub const DEFAULT_PORT: u16 = 7691;

/// Maximum time to wait for a client to send its request line.
const READ_TIMEOUT: Duration = Duration::from_secs(5);

/// Run the daemon, listening on `bind` (e.g. `"127.0.0.1:7691"`) until killed.
///
/// Binding a fixed port also enforces a single instance: a second `serve`
/// fails to bind and exits.
pub fn run_server(bind: &str) -> Result<(), ImSwitchError> {
    let listener = TcpListener::bind(bind)?;
    eprintln!("im-switch: serving on {bind}");
    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                if let Err(e) = handle_conn(s) {
                    eprintln!("im-switch: connection error: {e}");
                }
            }
            Err(e) => eprintln!("im-switch: accept error: {e}"),
        }
    }
    Ok(())
}

/// Read one request line, execute it, and write the response back.
///
/// Response format: line 1 is `ok` or `err: <message>`; for `ok`, the bytes
/// after the first newline are the verbatim stdout the CLI would have printed,
/// so the daemon's output matches the CLI byte-for-byte.
fn handle_conn(stream: TcpStream) -> std::io::Result<()> {
    stream.set_read_timeout(Some(READ_TIMEOUT))?;

    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let line = line.trim();
    if line.is_empty() {
        return Ok(());
    }

    let response = match dispatch_line(line) {
        Ok(payload) => format!("ok\n{payload}"),
        Err(e) => format!("err: {e}\n"),
    };

    let mut writer = stream;
    writer.write_all(response.as_bytes())?;
    writer.flush()?;
    Ok(())
}

/// Execute a single command line and return the stdout payload (possibly empty
/// or multi-line) that the equivalent CLI invocation would have printed.
fn dispatch_line(line: &str) -> Result<String, ImSwitchError> {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    match tokens.as_slice() {
        ["get"] => Ok(format!("{}\n", get_input_method()?)),
        ["set", im] => {
            set_input_method(im)?;
            Ok(String::new())
        }
        ["list"] => {
            let mut out = String::new();
            for method in list_input_methods()? {
                out.push_str(&method);
                out.push('\n');
            }
            Ok(out)
        }
        #[cfg(target_os = "windows")]
        ["ime", action] => dispatch_ime(action),
        #[cfg(not(target_os = "windows"))]
        ["ime", ..] => Err(ImSwitchError::Platform(
            "ime control is only supported on Windows".to_string(),
        )),
        _ => Err(ImSwitchError::Platform(format!("unknown command: {line}"))),
    }
}

#[cfg(target_os = "windows")]
fn dispatch_ime(action: &str) -> Result<String, ImSwitchError> {
    use im_switch::{get_ime_state, set_ime_state, toggle_ime_state};
    match action {
        "get" => Ok(format!("{}\n", if get_ime_state()? { "on" } else { "off" })),
        "on" => {
            set_ime_state(true)?;
            Ok(String::new())
        }
        "off" => {
            set_ime_state(false)?;
            Ok(String::new())
        }
        "toggle" => Ok(format!(
            "{}\n",
            if toggle_ime_state()? { "on" } else { "off" }
        )),
        _ => Err(ImSwitchError::Platform(format!(
            "unknown ime action: {action}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::thread;

    /// Send one raw request line to `bind` and return the full response.
    fn raw_request(bind: &str, request: &str) -> String {
        let mut stream = TcpStream::connect(bind).unwrap();
        stream.write_all(request.as_bytes()).unwrap();
        stream.write_all(b"\n").unwrap();
        stream.flush().unwrap();
        let mut resp = String::new();
        stream.read_to_string(&mut resp).unwrap();
        resp
    }

    /// Accept exactly one connection and serve it with `handle_conn`, returning
    /// the bound address and the server thread's join handle.
    fn serve_once() -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let handle = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            handle_conn(stream).unwrap();
        });
        (addr, handle)
    }

    #[test]
    fn unknown_command_returns_err() {
        let (addr, server) = serve_once();
        let resp = raw_request(&addr, "bogus");
        server.join().unwrap();
        assert!(resp.starts_with("err: "), "resp = {resp:?}");
        assert!(resp.contains("unknown command"), "resp = {resp:?}");
    }

    #[test]
    fn empty_request_writes_nothing() {
        let (addr, server) = serve_once();
        let resp = raw_request(&addr, "");
        server.join().unwrap();
        assert_eq!(resp, "");
    }

    #[test]
    fn dispatch_unknown_is_error() {
        assert!(dispatch_line("nope").is_err());
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn ime_is_unsupported_off_windows() {
        let err = dispatch_line("ime off").unwrap_err();
        assert!(format!("{err}").contains("only supported on Windows"));
    }
}
