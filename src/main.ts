import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

const WINDOW_TITLE_PREFIX = "Midnight Commander";

/** MC sets the terminal title via OSC 0, e.g. "mc [user@host]:~/path". */
function parseMcWorkingDir(oscTitle: string): string | null {
  const match = /^mc \[[^\]]+\]:(.+)$/.exec(oscTitle.trim());
  return match?.[1] ?? null;
}

function parseMcDirFromChunk(chunk: string): string | null {
  let latest: string | null = null;
  let searchFrom = 0;

  while (true) {
    const rel = chunk.indexOf("\x1b]0;", searchFrom);
    if (rel < 0) {
      break;
    }

    const payloadStart = searchFrom + rel + 4;
    const rest = chunk.slice(payloadStart);
    const belEnd = rest.indexOf("\x07");
    const stEnd = rest.indexOf("\x1b\\");
    const endCandidates = [belEnd, stEnd].filter((value) => value >= 0);
    if (endCandidates.length === 0) {
      break;
    }

    const end = Math.min(...endCandidates);
    const dir = parseMcWorkingDir(rest.slice(0, end));
    if (dir) {
      latest = dir;
    }
    searchFrom = payloadStart + end + 1;
  }

  return latest;
}

async function updateWindowTitle(appWindow: ReturnType<typeof getCurrentWebviewWindow>, dir: string) {
  await appWindow.setTitle(`${WINDOW_TITLE_PREFIX}: ${dir}`);
}

async function bootTerminal() {
  const appWindow = getCurrentWebviewWindow();
  const windowLabel = appWindow.label;

  const term = new Terminal({
    convertEol: true,
    cursorBlink: true,
    fontFamily: "Menlo, Monaco, 'Courier New', monospace",
    fontSize: 13,
    // mc is a fullscreen TUI; scrollback 0 stops FitAddon reserving ~14px for a scrollbar gutter.
    scrollback: 0,
    theme: {
      background: "#000000",
    },
  });

  const fitAddon = new FitAddon();
  term.loadAddon(fitAddon);
  term.open(document.getElementById("terminal")!);
  fitAddon.fit();

  term.onTitleChange((title) => {
    const dir = parseMcWorkingDir(title);
    if (dir) {
      void updateWindowTitle(appWindow, dir);
    }
  });

  await invoke("spawn_mc", {
    windowLabel,
    cols: term.cols,
    rows: term.rows,
  });

  const unlistenOutput = await listen<string>(`pty-output-${windowLabel}`, (event) => {
    const dir = parseMcDirFromChunk(event.payload);
    if (dir) {
      void updateWindowTitle(appWindow, dir);
    }
    term.write(event.payload);
  });

  const unlistenExit = await listen(`pty-exit-${windowLabel}`, () => {
    unlistenOutput();
    unlistenExit();
    term.dispose();
  });

  term.onData((data) => {
    void invoke("write_pty", { windowLabel, data });
  });

  term.onResize(({ cols, rows }) => {
    void invoke("resize_pty", { windowLabel, cols, rows });
  });

  const onBrowserResize = () => fitAddon.fit();
  globalThis.addEventListener("resize", onBrowserResize);
}

bootTerminal().catch((error) => {
  const message = error instanceof Error ? error.message : String(error);
  document.body.innerHTML = `<pre style="color:#f55;padding:1rem;">Failed to start mc: ${message}</pre>`;
});