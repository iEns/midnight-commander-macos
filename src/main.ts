import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

async function bootTerminal() {
  const appWindow = getCurrentWebviewWindow();
  const windowLabel = appWindow.label;

  const term = new Terminal({
    convertEol: true,
    cursorBlink: true,
    fontFamily: "Menlo, Monaco, 'Courier New', monospace",
    fontSize: 13,
    theme: {
      background: "#000000",
    },
  });

  const fitAddon = new FitAddon();
  term.loadAddon(fitAddon);
  term.open(document.getElementById("terminal")!);
  fitAddon.fit();

  await invoke("spawn_mc", {
    windowLabel,
    cols: term.cols,
    rows: term.rows,
  });

  const unlistenOutput = await listen<string>(`pty-output-${windowLabel}`, (event) => {
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