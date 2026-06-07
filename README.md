# ⌨️ im-switch

A cross-platform input method switcher — Rust crate + CLI.

[![Crates.io](https://img.shields.io/crates/v/im-switch)](https://crates.io/crates/im-switch)
[![docs.rs](https://img.shields.io/docsrs/im-switch)](https://docs.rs/im-switch)
[![CI](https://github.com/drop-stones/im-switch/actions/workflows/ci.yml/badge.svg)](https://github.com/drop-stones/im-switch/actions/workflows/ci.yml)

**im-switch** lets you query, switch, and list input methods from the command line or from Rust code. It is designed for tools that need to programmatically control input methods — for example, Vim/Neovim plugins that switch to an ASCII layout when leaving insert mode.

### Features

- **Cross-platform** — Linux, Windows, and macOS with a single unified API
- **Linux auto-detection** — automatically detects fcitx5 or ibus at runtime
- **Library + CLI** — use as a Rust crate (`cargo add im-switch`) or as a standalone CLI tool
- **Windows IME control** — get/set keyboard layout (KLID) and toggle IME on/off
- **Low-latency IPC mode** — run a loopback daemon and forward commands over TCP, avoiding per-call process startup (notably a Windows `.exe` from WSL2)

## Supported platforms

| Platform | Backend |
|----------|---------|
| Linux | fcitx5 (D-Bus), ibus (CLI) |
| Windows | Win32 API (keyboard layout + IME) |
| macOS | Carbon TIS API |

## Installation

```sh
cargo install im-switch
```

## CLI

| Command | Description |
|---------|-------------|
| `im-switch get` | Print the current input method |
| `im-switch set <id>` | Switch to the specified input method |
| `im-switch list` | List available input methods |

### Windows-only: IME control

| Command | Description |
|---------|-------------|
| `im-switch ime get` | Print IME state (`on` / `off`) |
| `im-switch ime on` | Turn on the IME |
| `im-switch ime off` | Turn off the IME |
| `im-switch ime toggle` | Toggle the IME state |

### Daemon / remote mode (IPC)

Spawning a fresh process per call can be slow — notably a Windows `.exe` invoked from WSL2. Run a long-lived loopback daemon and forward commands to it over TCP instead:

| Command | Description |
|---------|-------------|
| `im-switch serve [--addr 127.0.0.1] [--port 7691]` | Run a loopback daemon serving commands over TCP |
| `im-switch remote [--addr ADDR] <COMMAND>...` | Forward `COMMAND` to a running daemon (exit `1` = daemon reported an error, `2` = daemon unreachable) |

`COMMAND` is any of the commands above (`get`, `set <id>`, `list`, `ime …`); it is forwarded verbatim and executed by the daemon. `remote ping` checks reachability without touching the input method, and `remote shutdown` stops the daemon.

Example (WSL2 controlling the Windows IME):

```sh
im-switch serve              # on the Windows side
im-switch remote ime off     # from WSL2 — forwarded to the daemon
```

## Library

```sh
cargo add im-switch
```

See [docs.rs](https://docs.rs/im-switch) for API documentation.

## License

MIT
