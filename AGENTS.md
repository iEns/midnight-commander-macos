# Midnight Commander macOS App — Agent Guide

This project packages **GNU Midnight Commander (`mc`)** as a standalone macOS `.app`. It is **not** a native rewrite of mc. The app hosts the real Homebrew/system `mc` binary inside an embedded xterm.js terminal (Tauri 2 + portable-pty).

**Bundle ID:** `com.local.midnight-commander`  
**Built app path:** `src-tauri/target/release/bundle/macos/Midnight Commander.app`

---

## What works today

- Double-clickable macOS app with its own identity (not Terminal.app)
- Full mc UI: mouse, keyboard, resize, fullscreen, maximize, minimize
- Multi-window: **File → New Window** or **⌘N** (each window = one PTY + one `mc` process)
- Window closes when mc exits (F10 quit) or when the red close button is clicked
- Live window titles: `Midnight Commander: ~` (parsed from mc OSC 0 escape sequences)
- Custom MC-style icon (navy dual-pane + yellow status bar)

## What this app deliberately does NOT do

- Does **not** bundle `mc` inside the app — it resolves `mc` from the host machine at runtime
- Does **not** replicate shell-function wrappers (e.g. zsh `mc` aliases that persist directory between invocations)
- Does **not** code-sign or notarize (unsigned local build)

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  Midnight Commander.app (Tauri 2)                           │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  Webview (index.html → src/main.ts)                   │  │
│  │  xterm.js terminal + FitAddon                         │  │
│  │  - writes keystrokes → invoke("write_pty")            │  │
│  │  - listens pty-output-{label} → term.write()          │  │
│  │  - listens pty-exit-{label} → dispose + close         │  │
│  │  - onTitleChange / OSC parse → setTitle()             │  │
│  └───────────────────────────────────────────────────────┘  │
│                          ↕ Tauri IPC + events               │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  Rust backend (src-tauri/src/)                        │  │
│  │  PtySessionRegistry — one PTY session per window      │  │
│  │  - spawn_mc: open PTY, run mc, start reader thread    │  │
│  │  - reader: emit output, parse OSC 0, set_title        │  │
│  │  - on mc exit: emit pty-exit, close window            │  │
│  │  resolve_mc: find mc on disk (Homebrew paths)         │  │
│  └───────────────────────────────────────────────────────┘  │
│                          ↕ portable-pty                     │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  Real `mc` child process (TERM=xterm-256color)        │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### Window ↔ session mapping

| Window label | Created by | PTY session |
|---|---|---|
| `mc-main` | `tauri.conf.json` (startup window) | spawned on page load |
| `mc-{uuid}` | `new_mc_window` command (⌘N) | spawned on page load |

Each webview loads the same `index.html`. `getCurrentWebviewWindow().label` is the session key.

### Event protocol

| Event | Direction | Payload | Purpose |
|---|---|---|---|
| `pty-output-{label}` | Rust → frontend | `string` (raw PTY bytes as UTF-8 lossy) | Terminal output |
| `pty-exit-{label}` | Rust → frontend | `()` | mc process ended; frontend disposes xterm |

### Tauri commands (invoke)

| Command | Args | Purpose |
|---|---|---|
| `spawn_mc` | `windowLabel`, `cols`, `rows` | Create PTY + spawn mc |
| `write_pty` | `windowLabel`, `data` | Send keystrokes to mc |
| `resize_pty` | `windowLabel`, `cols`, `rows` | Resize PTY on terminal resize |
| `close_pty_session` | `windowLabel` | Kill mc + destroy session |
| `new_mc_window` | — | Open another mc window |
| `resolve_mc_path` | — | Debug: resolved mc binary path |
| `dry_run_mc` | `windowLabel` | Debug: what would be executed |
| `list_pty_sessions` | — | Debug: active sessions |
| `get_pty_session_count` | — | Debug: session count |

---

## Project layout

```
mc-app/
├── README.md                 ← user-facing setup and build guide
├── AGENTS.md                 ← this file
├── LICENSE                   ← MIT
├── index.html                ← single-page shell (#terminal div)
├── package.json              ← npm scripts, frontend deps
├── vite.config.ts            ← dev server port 1420
├── src/
│   ├── main.ts               ← xterm boot, IPC, title updates
│   └── styles.css            ← fullscreen black terminal
├── src-tauri/
│   ├── tauri.conf.json       ← app metadata, window defaults, icons
│   ├── capabilities/default.json  ← Tauri 2 permissions
│   ├── icons/                ← generated PNG + icon.icns
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs           ← entry; --verify-sessions CLI flag
│       ├── lib.rs            ← Tauri builder, menu, commands
│       ├── pty_registry.rs   ← PTY sessions, reader thread, lifecycle
│       ├── resolve_mc.rs     ← find mc binary on macOS
│       └── mc_title.rs       ← parse mc OSC 0 directory titles
├── scripts/
│   ├── generate-mc-icon.py   ← procedural icon generator
│   ├── verify-launch.sh      ← launch + multi-window smoke test
│   └── capture-tauri-evidence.sh
└── tests/
    └── run_tests.sh          ← runs cargo test
```

---

## Prerequisites

- **macOS** (primary target; Tauri is cross-platform but this app is macOS-focused)
- **Node.js** + npm
- **Rust** (edition 2021, `rust-version = "1.77"`)
- **Xcode CLI tools** (for `iconutil`, linking)
- **Homebrew `mc`:** `brew install midnight-commander`

Typical mc location: `/opt/homebrew/bin/mc`

---

## Commands

```bash
# Development (hot reload frontend, spawns Tauri window)
npm run tauri dev

# Production build → .app bundle
npm run tauri build

# Frontend only
npm run build
npm run dev

# Tests (requires mc installed — resolve_mc tests hit real binary)
npm test
# or
./tests/run_tests.sh
cargo test --manifest-path src-tauri/Cargo.toml

# Regenerate app icons (requires macOS iconutil)
python3 scripts/generate-mc-icon.py
# Then rebuild: npm run tauri build

# CLI session verification (no GUI)
src-tauri/target/release/midnight-commander --verify-sessions

# Live multi-window verification (launches GUI with flag)
MC_VERIFY_OUTPUT=/tmp/sessions.json \
  "src-tauri/target/release/bundle/macos/Midnight Commander.app/Contents/MacOS/midnight-commander" \
  --verify-live-multi-window
```

After rebuilding, launch from:
`src-tauri/target/release/bundle/macos/Midnight Commander.app`

If the Dock icon looks stale after an icon change: quit the app, then `killall Dock`.

---

## Key implementation details

### mc binary resolution (`resolve_mc.rs`)

Search order (first executable wins):

1. `/opt/homebrew/bin/mc`
2. `/usr/local/bin/mc`
3. `$HOME/.homebrew/bin/mc`
4. `$HOME/.homebrew/opt/midnight-commander/bin/mc`
5. `$PATH` entries (appended)

GUI apps have a minimal `PATH`; the hardcoded Homebrew paths are intentional.

To bundle mc inside the app later: change `create_session` to use a path relative to the app bundle instead of `resolve_mc()`.

### PTY environment (`pty_registry.rs`)

Spawned mc gets:

- `TERM=xterm-256color`
- `COLORTERM=truecolor`

### Window lifecycle

1. **Frontend loads** → `spawn_mc` with xterm cols/rows
2. **Reader thread** streams PTY output → `pty-output-{label}` events
3. **mc exits** → reader ends → `pty-exit-{label}` → `close_window_for_label`
4. **User closes window** → `WindowEvent::CloseRequested` → `destroy_session` (no frontend `onCloseRequested` listener — that previously blocked native close)
5. **App quit** → `RunEvent::ExitRequested` → destroy all sessions

### Live window titles (`mc_title.rs` + `src/main.ts`)

mc sets the terminal title via **OSC 0** when `[Layout] xterm_title=true` in mc config (default on).

Format: `mc [user@host]:~/path`

Title updates happen in **two places** (belt and suspenders):

1. **Rust PTY reader** — parses `\x1b]0;...` in raw chunks, calls `window.set_title()` (no frontend permission needed)
2. **Frontend** — `term.onTitleChange` + OSC parse in `pty-output` listener, calls `appWindow.setTitle()`

Frontend `setTitle` requires `core:window:allow-set-title` in `capabilities/default.json`. Without it, calls silently fail.

### Menu

Defined in `lib.rs` → `build_menu`:

- **Midnight Commander → Quit**
- **File → New Window** (⌘N)
- **File → Close Window**

---

## Common change recipes

### Change terminal appearance

Edit `src/main.ts` (`Terminal` options) and `src/styles.css`.

### Change default window size / title

Edit `src-tauri/tauri.conf.json` (`app.windows`) and `new_mc_window` in `lib.rs`.

### Change mc search paths

Edit `default_search_paths` in `resolve_mc.rs`. Add tests there.

### Add a new Tauri command

1. Implement in `lib.rs` (or a new module)
2. Register in `invoke_handler![...]`
3. If it needs permissions, add to `capabilities/default.json`
4. Call from frontend via `invoke("command_name", { ... })`

### Change the icon

Edit `scripts/generate-mc-icon.py` → run `python3 scripts/generate-mc-icon.py` → `npm run tauri build`.

**Icon gotcha:** When drawing per-pixel into PNG rows, byte index is `(x * 4)` within a row — **not** `(y * size + x) * 4`. Wrong indexing corrupts a scanline and shifts the bottom of the icon.

Palette: navy frame `(0,0,128)`, bright left pane `(85,85,255)`, dark right pane `(0,0,170)`, yellow status `(255,255,85)`.

### Add Tauri permissions

Edit `src-tauri/capabilities/default.json`. Check available permissions in `src-tauri/gen/schemas/` (regenerated on build). `core:window:default` includes `allow-title` (read) but **not** `allow-set-title` (write).

---

## mc user config notes

Config: `~/.config/mc/ini`

Relevant settings:

- `[Layout] xterm_title=true` — required for live window titles (OSC 0). User has this enabled.
- `[Layout] mouse_move_pages` etc. — normal mc behavior, passed through via PTY.

The app does not read mc config directly; mc handles it internally.

---

## Testing

**Rust unit tests** (`cargo test`):

| Module | Tests |
|---|---|
| `resolve_mc` | Finds real mc, empty PATH fallback, fake executable, not-found |
| `mc_title` | OSC payload parsing, chunk scanning |
| `pty_registry` | Session create/destroy, dry-run |

Integration scripts (manual / CI-style):

- `scripts/verify-launch.sh` — open bundle, count mc processes, live multi-window flag
- `scripts/capture-tauri-evidence.sh` — bundle structure + unit tests log

---

## Known gotchas (bugs already fixed)

| Issue | Cause | Fix |
|---|---|---|
| `window.addEventListener` crash | `window` variable shadowed Tauri `WebviewWindow` | Use `globalThis.addEventListener` |
| Red close button / black window after F10 | Frontend `onCloseRequested` blocked native close | Removed listener; Rust closes on PTY exit |
| Window title never updates | Missing `core:window:allow-set-title` | Added permission + Rust-side `set_title` |
| Icon bottom shifted right | Icon script used `(cy*size+x)*4` row index | Use `(x*4)` within row bytes |
| AppleScript launcher path quoting | Homebrew version in path broke `osascript -e` | Superseded — old shell launcher removed |

---

## Git

```bash
git log --oneline
# 326d9fd Add live window titles from mc OSC and fix icon alignment
# 1f69fa1 Add Tauri macOS app that hosts real mc in xterm.js
```

Build artifacts and local noise are gitignored: `node_modules/`, `dist/`, `src-tauri/target/`, `src-tauri/gen/`, `__pycache__/`, `.env*`, editor folders.

---

## Future ideas (not implemented)

- **Code signing / notarization** for distribution outside the build machine
- **Bundle `mc` inside the app** for machines without Homebrew
- **Install to /Applications** post-build script
- **Copy last directory on quit** via `mc -P /tmp/mc-printwd` (like shell wrapper functions)
- **Preference pane** for mc path override, font size, theme

---

## Dependencies

| Layer | Packages |
|---|---|
| Frontend | `@tauri-apps/api`, `@xterm/xterm`, `@xterm/addon-fit`, `vite`, `typescript` |
| Backend | `tauri 2`, `portable-pty`, `serde`, `uuid`, `thiserror` |
| System | Homebrew `mc`, macOS `iconutil` (icon generation) |

---

## Quick debugging

```bash
# Where does the app find mc?
cargo test -p midnight-commander resolve_mc::tests::resolves_real_mc -- --nocapture

# Resolved path from running app (dev tools console):
# await window.__TAURI__.core.invoke('resolve_mc_path')

# List active sessions:
# await window.__TAURI__.core.invoke('list_pty_sessions')
```

If mc fails to start, the frontend shows a red error banner from the `bootTerminal().catch()` handler in `src/main.ts`.