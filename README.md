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

## Library

```sh
cargo add im-switch
```

See [docs.rs](https://docs.rs/im-switch) for API documentation.

## License

MIT
