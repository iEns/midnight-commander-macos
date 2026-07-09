# Midnight Commander for macOS

A native macOS application that runs [GNU Midnight Commander](https://midnight-commander.org/) (`mc`) in its own window — not inside Terminal.app.

This is **not** a rewrite of mc. The app embeds the real `mc` binary from your system inside an xterm.js terminal, backed by a [Tauri 2](https://v2.tauri.app/) shell and a Rust PTY layer.

## Features

- Double-clickable `.app` with a custom MC-style icon
- Full mc UI: keyboard, mouse, resize, fullscreen, maximize, minimize
- Multi-window support (**File → New Window** or **⌘N**)
- Live window titles synced from mc (`Midnight Commander: ~/path`)
- Window closes when mc exits (F10) or when you click the red close button

## Requirements

| Tool | Purpose |
|------|---------|
| **macOS** | Primary target (10.13+) |
| **Node.js** + npm | Frontend tooling and Tauri CLI |
| **Rust** 1.77+ | Tauri backend (`rustup` recommended) |
| **Xcode Command Line Tools** | Linking and `iconutil` (for icon regeneration) |
| **Homebrew `mc`** | The file manager this app hosts |

Install mc if you do not already have it:

```bash
brew install midnight-commander
```

The app resolves `mc` at runtime from common Homebrew paths and `$PATH`. It does **not** bundle mc inside the `.app`.

## Quick start

```bash
git clone https://github.com/iEns/midnight-commander-macos.git
cd midnight-commander-macos
npm install
npm run tauri build
```

Open the built application:

```text
src-tauri/target/release/bundle/macos/Midnight Commander.app
```

Drag it to `/Applications` if you like. The build is **unsigned** (local development build; Gatekeeper may require right-click → Open the first time).

## Development

Start the app with hot-reloaded frontend:

```bash
npm run tauri dev
```

Other useful commands:

```bash
npm run build          # Vite frontend only
npm run dev            # Vite dev server only (port 1420)
npm test               # Rust unit tests (requires mc installed)
```

### Regenerate icons

Icons are committed so a fresh clone can build immediately. To regenerate them after editing the generator:

```bash
python3 scripts/generate-mc-icon.py
npm run tauri build
```

If the Dock icon looks stale after a change, quit the app and run `killall Dock`.

## Testing

```bash
npm test
# or
./tests/run_tests.sh
```

Rust tests cover mc binary resolution, OSC title parsing, and PTY session lifecycle. The `resolve_mc` tests expect a real `mc` binary on the machine.

Optional integration scripts (manual / smoke tests):

```bash
./scripts/verify-launch.sh
./scripts/capture-tauri-evidence.sh
```

## How it works

```text
┌──────────────────────────────────────────────┐
│  Midnight Commander.app (Tauri 2)            │
│  ┌────────────────────────────────────────┐  │
│  │  Webview: xterm.js + FitAddon          │  │
│  │  keystrokes → write_pty                │  │
│  │  pty-output → term.write()             │  │
│  └────────────────────────────────────────┘  │
│              ↕ Tauri IPC + events            │
│  ┌────────────────────────────────────────┐  │
│  │  Rust: PtySessionRegistry              │  │
│  │  spawn mc in PTY, stream I/O, titles   │  │
│  └────────────────────────────────────────┘  │
│              ↕ portable-pty                  │
│  ┌────────────────────────────────────────┐  │
│  │  System `mc` (TERM=xterm-256color)     │  │
│  └────────────────────────────────────────┘  │
└──────────────────────────────────────────────┘
```

Each window maps to one PTY session and one `mc` process. The startup window label is `mc-main`; new windows get `mc-{uuid}`.

### mc search order

The backend looks for an executable `mc` in this order:

1. `/opt/homebrew/bin/mc`
2. `/usr/local/bin/mc`
3. `$HOME/.homebrew/bin/mc`
4. `$HOME/.homebrew/opt/midnight-commander/bin/mc`
5. Entries on `$PATH`

GUI apps receive a minimal `PATH` on macOS, so the hardcoded Homebrew paths are intentional.

## Project layout

```text
mc-app/
├── README.md              # This file
├── AGENTS.md              # Architecture and maintainer guide
├── src/                   # xterm.js frontend (TypeScript)
├── src-tauri/             # Tauri + Rust backend
├── scripts/               # Icon generator and smoke-test scripts
└── tests/                 # Test runner
```

## What this app does not do

- Bundle `mc` — you need Homebrew (or another install) on the host machine
- Replicate shell `mc` wrapper functions that persist the last directory between invocations
- Code-sign or notarize the `.app` (unsigned local builds only today)

## Troubleshooting

**mc fails to start** — Install it with `brew install midnight-commander`. The frontend shows a red error banner if spawn fails.

**Window title does not update** — mc sends titles via OSC 0 when `[Layout] xterm_title=true` in `~/.config/mc/ini` (enabled by default).

**Where does the app find mc?**

```bash
cargo test -p midnight-commander resolve_mc::tests::resolves_real_mc -- --nocapture
```

## Contributing

Bug reports and pull requests are welcome. For architecture details, IPC contracts, and common change recipes, see [AGENTS.md](AGENTS.md).

## License

[MIT](LICENSE)
