# WSL2 IPC Design — speeding up IME switching under WSL2

Status: **spec agreed** (implementation not started)
Last updated: 2026-06-04

## 1. Problem

`im-switch.zellij` is a Zellij WASM plugin. The plugin itself cannot open
sockets; it can only issue `run_command`, which the Zellij host executes on the
machine where Zellij runs — i.e. **inside WSL2**. On WSL2 the plugin runs the
Windows binary `im-switch.exe ime <on|off|get>` to control the Windows IME.
`im-switch.nvim` is in the same situation.

Spawning a Windows `.exe` from WSL2 goes through the WSL interop layer and is
slow. This is especially painful for the `Restore` strategy, which performs a
`get` followed by a `set` on each transition.

## 2. Measurements (2026-06-04, Win11 25H2 build 26200)

| Path | median |
|------|--------|
| Native Linux process spawn (`uname`) | 1.1 ms |
| Windows interop spawn only (`cmd.exe /c exit`) | 22.8 ms |
| `im-switch.exe ime get` | 64.2 ms |
| `im-switch.exe ime off` | 62.7 ms |
| `im-switch.exe --version` (no IME work) | 63.2 ms |
| WSL2 → Windows TCP round-trip, NAT via gateway | 0.36 ms |
| WSL2 → Windows TCP round-trip, **mirrored via 127.0.0.1** | 0.37 ms |

Key findings:

- **The IME operation itself costs ~0 ms.** `--version` (which does no IME work)
  takes the same ~63 ms as `ime off`.
- The ~63 ms is entirely **Windows `.exe` startup cost**, split into roughly:
  - ~22 ms — WSL2→Windows interop process creation
  - ~40 ms — `im-switch.exe`'s own load (Rust runtime + `windows` crate / DLL init)
- A persistent daemon removes both costs per call. Expected per-call cost becomes
  `native client spawn (~1 ms) + TCP RT (~0.4 ms) ≈ 1.5 ms` — roughly **40× faster**.
  The `Restore` get+set (~126 ms) would drop to ~3 ms.
- Loopback reachability under mirrored mode was **verified** (with
  `[experimental] hostAddressLoopback=true`): 0.37 ms round-trip, **no firewall
  popup** (loopback traffic is never filtered by Windows Firewall).

So the fix is conceptually simple: **stop spawning a Windows `.exe` per switch.**

## 3. Networking background (why this is subtle)

WSL2 runs in a lightweight VM. From inside WSL2, `localhost` (127.0.0.1) means
the **Linux side**, not Windows.

- **NAT mode (default):** Windows is a separate host reachable via the default
  gateway (e.g. `192.168.192.1`). That gateway IP **changes on every WSL
  restart**. A daemon listening on a non-loopback interface also triggers a
  **Windows Firewall popup** the first time it listens.
- **Mirrored mode (Win11 22H2+, available here on 25H2):** WSL2 shares the
  Windows network stack. Windows is reachable at `127.0.0.1`, and **loopback
  traffic is never filtered by Windows Firewall** — so no popup, ever. No dynamic
  IP. Simpler client.

Mirrored mode is enabled globally via `.wslconfig` (requires `wsl --shutdown`):

```ini
# C:\Users\<user>\.wslconfig
[wsl2]
networkingMode=mirrored

[experimental]
hostAddressLoopback=true
```

## 4. Chosen approach: mirror-mode-only IPC fast path

Implement a daemon that binds **loopback only**, and use it **only under mirrored
mode**. When the fast path is unavailable, **fall back to the current behavior**
(direct `im-switch.exe` invocation). This is a strict improvement with no
regression for non-mirrored setups.

```
[zellij plugin]  --run_command-->  [native Linux client]  --TCP 127.0.0.1-->  [Windows daemon]
   (WASM)            ~1 ms spawn       (Linux im-switch)                       (Windows im-switch.exe serve)
```

### No new binaries — just two new run-modes

We do **not** ship separate client/server executables. The existing single
`im-switch` crate gains two run-modes, available on every platform:

- **server mode** — `im-switch serve` (e.g. `im-switch.exe serve` on Windows)
- **client mode** — the normal CLI, when `IM_SWITCH_REMOTE` is set, forwards the
  command over TCP instead of acting locally.

So the Linux `im-switch` binary is dual-purpose: native fcitx5/ibus control when
`IM_SWITCH_REMOTE` is unset, remote client when it is set (the plugin sets it
only on WSL2). The release asset matrix is unchanged (linux / macos / windows ×
arch). **WSL2 is the only case that uses two binaries together** (the existing
Linux build as client + the existing Windows build as server/fallback).

### Why opt-in (not auto-detecting mirror mode)

Robustly detecting mirror vs NAT from the Linux side is fragile, and naively
"try then start daemon" would spawn orphan daemons on Windows under NAT.
Enabling mirrored mode is already a deliberate user action, so requiring one
extra plugin option (`wsl2_ipc`) is acceptable. The **connection attempt itself
doubles as the mirror-reachability check**: under NAT, `127.0.0.1:PORT` never
reaches the Windows daemon, so the client just fails and the plugin falls back.

## 5. Components (in the `im-switch` crate)

### 5.1 Server mode — `im-switch serve [--port N] [--addr A]`

- Binds `127.0.0.1:PORT` (default 7691, configurable).
- Cross-platform; dispatches to the existing `im_switch` library functions.
- Single-threaded accept loop is sufficient (very low call volume).
- Singleton by construction: a second instance fails to bind the fixed port and
  exits.
- **No idle self-exit** (decided): a loopback accept loop is near-zero cost, and
  on-demand restart by the client handles the rest.
- **Graceful `shutdown`** for manual/dev convenience. It is *not* the upgrade
  mechanism: a stale (old) daemon may not understand a newer command or may be
  wedged, so consumers still use `taskkill` + relaunch (§7/§11) when they must
  guarantee the daemon is gone regardless of its state.

Line protocol:

- Request = one line, the CLI args joined by spaces:
  `ime get|on|off|toggle`, `get`, `set <id>`, `list`, `shutdown`.
- Response = line 1 is `ok` or `err: <message>`; for `ok`, the bytes after the
  first newline are the **verbatim CLI stdout** (so daemon output == CLI output).
- `shutdown` replies `ok` and then stops the accept loop.

### 5.2 Client mode — triggered by `IM_SWITCH_REMOTE`

- Intercepted in `main()` **before** clap parsing, so it forwards raw argv. This
  sidesteps the fact that the `ime` subcommand is `cfg(windows)`-only — the Linux
  client can still forward `ime off`.
- `IM_SWITCH_REMOTE` value: `host:port`, bare `port` (→ `127.0.0.1:port`), or
  `auto`/empty (→ `127.0.0.1:7691`).
- Exit codes (so the plugin can react precisely):
  - `0` — success (payload printed to stdout)
  - `1` — daemon reachable but the operation failed
  - `2` — transport failure (daemon unreachable / refused)
- The client stays **thin**: on failure it just exits `2`. It does **not** start
  the daemon or fall back itself — that is the consumer's job (see §7).
- `im-switch serve` is never forwarded (run locally even if the env is set);
  args beginning with `-` (e.g. `--help`) are also run locally.

## 6. Binary layout (Plan B — server lives on the Windows filesystem)

The server's executable **must reside on the Windows filesystem**, not inside the
WSL VM. If the `.exe` lived under the WSL home (`~/.local/share`, i.e. the WSL
9P share), a long-lived daemon would be coupled to the WSL VM lifecycle: a
`wsl --shutdown`/restart could crash it or leave it stuck, and it could even
block clean WSL shutdown. Putting it on `C:` decouples it entirely.

| Role | Filesystem | Location |
|------|-----------|----------|
| Linux client | WSL FS | `~/.local/share/im-switch.zellij/im-switch` |
| Windows server + fallback | Windows FS | `<windows-data-dir>/im-switch.zellij/im-switch.exe` |

`<windows-data-dir>` resolution (use the **Windows-side** env, queried via
interop — never the WSL/Linux `XDG_DATA_HOME`, which would point back into the WSL
FS and re-introduce the coupling problem above):

1. Windows `%XDG_DATA_HOME%` if set, else
2. `%LOCALAPPDATA%` (= `C:\Users\<user>\AppData\Local`).

This matches the location the plugin already uses for native-Windows installs.
From WSL, resolve via `cmd.exe /c echo %LOCALAPPDATA%` (and `%XDG_DATA_HOME%`),
then convert with `wslpath -u` to a `/mnt/c/...` path for placement.

## 7. Lifecycle & orchestration (consumer's responsibility)

Consumers are `im-switch.zellij` and `im-switch.nvim`. The daemon is **shared**
and **on-demand**; **no consumer is responsible for stopping it** (stopping on
plugin exit would break a second consumer running in parallel).

Per IME switch, the consumer:

1. Runs the Linux client (fast path) with `IM_SWITCH_REMOTE` set.
2. On exit code `2` (daemon down):
   - run the Windows `im-switch.exe <args>` directly so this switch is not lost
     (current behavior), and
   - spawn `im-switch.exe serve` detached so the next call is fast.

Both binary paths are known to the consumer (it installed them), so all path /
startup knowledge stays in the consumer; the `im-switch` crate stays simple.

## 8. Security

Loopback-only bind means the daemon is reachable solely by local processes on the
same machine (shared loopback under mirrored mode). No LAN exposure, no firewall
rule, no auth token needed.

## 9. Distribution

- `im-switch`: add `serve` + client mode; cut a release; bump
  `REQUIRED_CLI_VERSION` in the consumers. Release assets unchanged.
- Consumers: on WSL2, install **two** existing assets — the Linux build (client,
  to WSL FS) and the Windows build (server/fallback, to the Windows data dir) —
  and wire up the orchestration in §7.

## 10. Staged implementation plan

1. `im-switch`: `serve` subcommand (loopback daemon + line protocol).
2. `im-switch`: client mode (`IM_SWITCH_REMOTE`) — forward / parse / exit codes.
3. `im-switch.zellij`: install both binaries on WSL2 (client → WSL FS, server →
   Windows data dir); add `wsl2_ipc` config; wire the §7 orchestration.
4. (later) `im-switch.nvim`: same orchestration.
5. Validate: measure under mirrored mode; confirm no regression under NAT.

## 11. Open questions

- Confirm default port (currently 7691).
- Protocol versioning: whether to add a `hello`/version line so a client can
  detect a **stale daemon after an upgrade** and have the consumer restart it
  (e.g. distinct exit code → `taskkill` + relaunch). Likely a v1.1 concern.
- `set <id>` with identifiers containing spaces (space-join protocol limitation);
  fine for KLIDs, revisit if Linux IM names need it.
