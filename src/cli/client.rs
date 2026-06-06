//! Remote client mode: forward a command line over TCP to an `im-switch serve`
//! daemon instead of running it locally. Triggered by the `--remote` flag, which
//! is intercepted in `main()` *before* clap parsing — so it can forward even the
//! `ime` subcommand, which is `cfg(windows)`-only in the local CLI.
//!
//! The client stays thin: on any failure it just returns a non-zero exit code.
//! Starting the daemon and falling back to a direct `im-switch.exe` call is the
//! consumer's job. See `docs/wsl2-ipc-design.md` §5.2 / §7.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use crate::cli::ipc::DEFAULT_PORT;

/// Time to wait for the TCP connection to establish. Kept short on purpose:
/// under WSL2 mirrored mode a dead port does not refuse instantly — the SYN is
/// dropped — so without this bound every call while the daemon is down would
/// hang for seconds. The daemon, when up, connects in well under 3ms.
const CONNECT_TIMEOUT: Duration = Duration::from_millis(200);

/// Time to wait for the request/response I/O once connected.
const IO_TIMEOUT: Duration = Duration::from_secs(5);

/// Exit code: daemon was reached but the operation failed.
pub const EXIT_OP_FAILED: i32 = 1;
/// Exit code: could not talk to the daemon (unreachable / malformed reply).
pub const EXIT_TRANSPORT: i32 = 2;

/// If `--remote` / `--remote=ADDR` appears in `args` (skipping argv[0]), return
/// the resolved daemon address and the remaining args to forward. Otherwise
/// `None`, meaning "run locally".
pub fn extract_remote(args: &[String]) -> Option<(String, Vec<String>)> {
    let mut found = false;
    let mut addr: Option<String> = None;
    let mut rest = Vec::new();
    for arg in args.iter().skip(1) {
        if arg == "--remote" {
            found = true;
        } else if let Some(value) = arg.strip_prefix("--remote=") {
            found = true;
            addr = Some(resolve_addr(value));
        } else {
            rest.push(arg.clone());
        }
    }
    found.then(|| (addr.unwrap_or_else(default_addr), rest))
}

/// Forward `cmd_args` to the daemon at `addr`; return the process exit code.
pub fn forward(addr: &str, cmd_args: &[String]) -> i32 {
    let request = cmd_args.join(" ");
    match send(addr, &request) {
        Ok(resp) => emit(&resp),
        Err(e) => {
            eprintln!("im-switch: cannot reach daemon at {addr}: {e}");
            EXIT_TRANSPORT
        }
    }
}

fn default_addr() -> String {
    format!("127.0.0.1:{DEFAULT_PORT}")
}

/// Resolve a `--remote=` value: empty → default, `host:port` as-is, bare digits
/// → `127.0.0.1:port`, bare host → `host:DEFAULT_PORT`.
fn resolve_addr(value: &str) -> String {
    if value.is_empty() {
        default_addr()
    } else if value.contains(':') {
        value.to_string()
    } else if value.bytes().all(|b| b.is_ascii_digit()) {
        format!("127.0.0.1:{value}")
    } else {
        format!("{value}:{DEFAULT_PORT}")
    }
}

/// Connect, send one request line, and read the full response.
fn send(addr: &str, request: &str) -> std::io::Result<String> {
    let sock = addr
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| std::io::Error::other("could not resolve address"))?;
    let mut stream = TcpStream::connect_timeout(&sock, CONNECT_TIMEOUT)?;
    stream.set_read_timeout(Some(IO_TIMEOUT))?;
    stream.set_write_timeout(Some(IO_TIMEOUT))?;
    stream.write_all(request.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()?;
    stream.shutdown(std::net::Shutdown::Write)?;
    let mut resp = String::new();
    stream.read_to_string(&mut resp)?;
    Ok(resp)
}

/// Print the response the way the local CLI would and return its exit code.
fn emit(resp: &str) -> i32 {
    let (code, out, err) = interpret(resp);
    print!("{out}");
    eprint!("{err}");
    code
}

/// Map a daemon response to `(exit_code, stdout, stderr)`. Pure, for testing.
fn interpret(resp: &str) -> (i32, String, String) {
    match resp.split_once('\n') {
        Some(("ok", payload)) => (0, payload.to_string(), String::new()),
        Some((first, _)) if first.starts_with("err: ") => {
            (EXIT_OP_FAILED, String::new(), format!("Error: {}\n", &first[5..]))
        }
        _ => (EXIT_TRANSPORT, String::new(), String::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn no_flag_runs_local() {
        let args = vec!["im-switch".into(), "ime".into(), "off".into()];
        assert!(extract_remote(&args).is_none());
    }

    #[test]
    fn bare_flag_uses_default_addr() {
        let args = vec!["im-switch".into(), "--remote".into(), "ime".into(), "off".into()];
        let (addr, rest) = extract_remote(&args).unwrap();
        assert_eq!(addr, "127.0.0.1:7691");
        assert_eq!(rest, vec!["ime".to_string(), "off".to_string()]);
    }

    #[test]
    fn flag_with_bare_port() {
        let args = vec!["im-switch".into(), "--remote=9000".into(), "get".into()];
        let (addr, rest) = extract_remote(&args).unwrap();
        assert_eq!(addr, "127.0.0.1:9000");
        assert_eq!(rest, vec!["get".to_string()]);
    }

    #[test]
    fn flag_with_host_port() {
        let args = vec!["im-switch".into(), "--remote=192.168.0.5:7000".into(), "get".into()];
        let (addr, _) = extract_remote(&args).unwrap();
        assert_eq!(addr, "192.168.0.5:7000");
    }

    #[test]
    fn interpret_ok_with_payload() {
        assert_eq!(interpret("ok\noff\n"), (0, "off\n".to_string(), String::new()));
    }

    #[test]
    fn interpret_ok_empty() {
        assert_eq!(interpret("ok\n"), (0, String::new(), String::new()));
    }

    #[test]
    fn interpret_err_maps_to_one() {
        let (code, out, err) = interpret("err: unknown command: bogus\n");
        assert_eq!(code, EXIT_OP_FAILED);
        assert!(out.is_empty());
        assert_eq!(err, "Error: unknown command: bogus\n");
    }

    #[test]
    fn interpret_malformed_is_transport() {
        assert_eq!(interpret(""), (EXIT_TRANSPORT, String::new(), String::new()));
    }

    #[test]
    fn send_reads_full_response() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let server = thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut buf = [0u8; 64];
            let _ = s.read(&mut buf);
            s.write_all(b"ok\nhello\n").unwrap();
        });
        let resp = send(&addr, "get").unwrap();
        server.join().unwrap();
        assert_eq!(resp, "ok\nhello\n");
    }

    #[test]
    fn send_errors_when_daemon_down() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        drop(listener); // free the port so the connect is refused
        assert!(send(&addr, "get").is_err());
    }

    #[test]
    fn forward_returns_zero_on_ok() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let server = thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut buf = [0u8; 64];
            let _ = s.read(&mut buf);
            s.write_all(b"ok\n").unwrap();
        });
        let code = forward(&addr, &["ime".to_string(), "off".to_string()]);
        server.join().unwrap();
        assert_eq!(code, 0);
    }
}
