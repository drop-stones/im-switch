# WSL2 IPC Design — speeding up IME switching under WSL2

Status: **implemented** (`im-switch` crate side; consumer auto-download pending)
Last updated: 2026-06-07

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
- **client mode** — `im-switch remote <command...>` forwards the command over TCP
  instead of acting locally.

So the Linux `im-switch` binary is dual-purpose: native fcitx5/ibus control via
the normal subcommands, remote client via `remote` (the plugin uses `remote`
only on WSL2). The release asset matrix is unchanged (linux / macos / windows ×
arch). **WSL2 is the only case that uses two binaries together** (the existing
Linux build as client + the existing Windows build as server/fallback).

### Why opt-in (not auto-detecting mirror mode)

Robustly detecting mirror vs NAT from the Linux side is fragile, and naively
"try then start daemon" would spawn orphan daemons on Windows under NAT.
Enabling mirrored mode is already a deliberate user action, so requiring one
extra plugin option (`wsl2_server`) is acceptable. The **connection attempt
itself doubles as the mirror-reachability check**: under NAT, `127.0.0.1:PORT`
never reaches the Windows daemon, so the client just fails and the plugin falls
back.

## 5. Components (in the `im-switch` crate)

### 5.1 Server mode — `im-switch serve [--port N] [--addr A]`

- Binds `127.0.0.1:PORT` (default 7691, configurable).
- **Loopback-only is enforced**: a non-loopback `--addr` (e.g. `0.0.0.0`) is
  rejected, since the daemon has no authentication.
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
  `ping`, `ime get|on|off|toggle`, `get`, `set <id>`, `list`, `shutdown`.
- Response = line 1 is `ok` or `err: <message>`; for `ok`, the bytes after the
  first newline are the **verbatim CLI stdout** (so daemon output == CLI output).
- `ping` replies `ok` with no payload and does not touch the IME — consumers use
  it to probe reachability (mirrored-mode fast-path detection, see §5.2).
- `shutdown` replies `ok` and then stops the accept loop.

### 5.2 Client mode — the `remote` subcommand

- `im-switch remote [--addr ADDR] <command...>` forwards `<command...>` to the
  daemon. The trailing args are captured verbatim (`trailing_var_arg`), so clap
  never parses them locally — the Linux client can forward `ime off` even though
  the `ime` subcommand is `cfg(windows)`-only. (An earlier design used a
  `--remote` flag intercepted before clap; the subcommand is cleaner — it needs
  no pre-clap hack and appears in `--help`.)
- `--addr ADDR` value: `host:port`, bare `port` (→ `127.0.0.1:port`), bare
  `host` (→ `host:7691`); omitted → `127.0.0.1:7691`.
- **Connect timeout = 200 ms** (a short, fixed bound). Under mirrored mode a
  *dead* loopback port does **not** refuse instantly — the SYN is dropped — so
  without this bound a down daemon would hang for seconds. When the daemon is up,
  connect is <3 ms, so 200 ms is a ~70x safety margin. The penalty only hits the
  first switch after the daemon dies; the consumer's fallback then respawns it.
- Exit codes (so the plugin can react precisely):
  - `0` — success (payload printed to stdout)
  - `1` — daemon reachable but the operation failed
  - `2` — transport failure (daemon unreachable / timed out / malformed reply)
- The client stays **thin**: on failure it just exits `2`. It does **not** start
  the daemon or fall back itself — that is the consumer's job (see §7).

## 6. Binary layout (server lives in the WSL install dir)

Both binaries live **in the same WSL install dir**, side by side:

| Role | Filesystem | Location |
|------|-----------|----------|
| Linux client | WSL FS | `~/.local/share/im-switch.zellij/im-switch` |
| Windows server + fallback | WSL FS | `~/.local/share/im-switch.zellij/im-switch.exe` |

An earlier plan (Plan B) put the server on the Windows filesystem
(`<windows-data-dir>/...`), out of concern that a daemon backed by the WSL 9P
share would be coupled to the WSL VM lifecycle. Investigation showed that concern
does **not** actually favor Windows-FS:

- The daemon is launched via interop and is a **child of `wsl.exe`**, so it is
  tied to the WSL session and **dies cleanly on `wsl --shutdown` regardless of
  where the `.exe` lives** — no orphan/zombie either way. (`setsid` only detaches
  it from the launching shell, not from the WSL VM.)
- Steady-state switch latency is **identical** (TCP to the resident daemon); the
  exe location only affects the one-time, background **cold start** — measured
  ~110 ms (WSL-FS) vs ~57 ms (Windows-FS), a +53 ms one-off that is invisible in
  practice.

WSL-FS placement is chosen because it **eliminates the fragile Windows-data-dir
resolution** (querying `%LOCALAPPDATA%` via `cmd.exe` + `wslpath`, which broke
under zellij's minimal `PATH`) and keeps everything in one directory derived from
`$HOME`.

## 7. Lifecycle & orchestration (consumer's responsibility)

Consumers are `im-switch.zellij` and `im-switch.nvim`. The daemon is **shared**
and **on-demand**; **no consumer is responsible for stopping it** (stopping on
plugin exit would break a second consumer running in parallel).

Per IME switch, the consumer:

1. Runs the Linux client fast path: `im-switch remote ime <arg>`.
2. On exit code `2` (daemon down), runs a **self-contained fallback command**
   that invokes the Windows `im-switch.exe ime <arg>` directly (so the switch is
   not lost) *and* restarts the daemon detached, in one shell command:
   `setsid '<server>' serve >/dev/null 2>&1 </dev/null & exec '<server>' ime <arg>`.
   The daemon is also pre-warmed once when the consumer becomes ready.

Both binary paths are known to the consumer (it installed them), so all path /
startup knowledge stays in the consumer; the `im-switch` crate stays simple. The
consumer's retry is generic ("on transport failure, run the stashed fallback
command once") and knows nothing about the daemon or `remote`.

## 8. Security

Loopback-only bind means the daemon is reachable solely by local processes on the
same machine (shared loopback under mirrored mode). No LAN exposure, no firewall
rule, no auth token needed.

## 9. Distribution

- `im-switch`: add `serve` + `remote`; cut a release (0.2.0); bump
  `REQUIRED_CLI_VERSION` in the consumers. Release assets unchanged.
- Consumers: on WSL2, install **two** existing assets into the same WSL install
  dir — the Linux build (client) and the Windows build (server/fallback) — and
  wire up the orchestration in §7.

## 10. Staged implementation plan

1. ✅ `im-switch`: `serve` subcommand (loopback daemon + line protocol + `shutdown`).
2. ✅ `im-switch`: client mode (`remote` subcommand) — forward / parse / exit codes.
3. 🔶 `im-switch.zellij`: `wsl2_server` config + §7 orchestration wired (done);
   both binaries currently **pre-placed by hand** in the WSL install dir —
   auto-download still pending.
4. (later) `im-switch.nvim`: same orchestration.
5. Validate: measure under mirrored mode; confirm no regression under NAT.

## 11. Open questions

- Default port is **7691** (confirmed).
- Protocol versioning: whether to add a `hello`/version line so a client can
  detect a **stale daemon after an upgrade** and have the consumer restart it
  (e.g. distinct exit code → `taskkill` + relaunch). Likely a v1.1 concern.
- `set <id>` with identifiers containing spaces (space-join protocol limitation);
  fine for KLIDs, revisit if Linux IM names need it.
