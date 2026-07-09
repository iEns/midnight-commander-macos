use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Emitter, Manager};

use crate::mc_title::{format_window_title, parse_mc_dir_from_chunk};
use crate::resolve_mc::{resolve_mc, ResolveMcError};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PtySessionInfo {
    pub session_id: String,
    pub window_label: String,
    pub mc_path: String,
}

struct PtySession {
    info: PtySessionInfo,
    master: Arc<Mutex<Box<dyn MasterPty + Send>>>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    child: Arc<Mutex<Box<dyn portable_pty::Child + Send + Sync>>>,
}

#[derive(Default)]
pub struct PtySessionRegistry {
    sessions: Mutex<HashMap<String, PtySession>>,
}

impl PtySessionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn session_count(&self) -> usize {
        self.sessions.lock().expect("lock sessions").len()
    }

    pub fn list_sessions(&self) -> Vec<PtySessionInfo> {
        self.sessions
            .lock()
            .expect("lock sessions")
            .values()
            .map(|session| session.info.clone())
            .collect()
    }

    pub fn has_session(&self, window_label: &str) -> bool {
        self.sessions
            .lock()
            .expect("lock sessions")
            .contains_key(window_label)
    }

    pub fn dry_run_for_window(&self, window_label: &str) -> Result<String, ResolveMcError> {
        let mc_path = resolve_mc()?;
        Ok(format!(
            "Would execute: {} for window {}",
            mc_path.display(),
            window_label
        ))
    }

    pub fn create_session(
        &self,
        app: Option<AppHandle>,
        window_label: String,
        cols: u16,
        rows: u16,
    ) -> Result<PtySessionInfo, String> {
        let mc_path = resolve_mc().map_err(|err| err.to_string())?;
        self.create_session_with_command(app, window_label, mc_path, Vec::new(), cols, rows)
    }

    pub fn create_session_with_command(
        &self,
        app: Option<AppHandle>,
        window_label: String,
        command_path: PathBuf,
        args: Vec<String>,
        cols: u16,
        rows: u16,
    ) -> Result<PtySessionInfo, String> {
        let mut sessions = self.sessions.lock().map_err(|_| "session lock poisoned")?;

        if sessions.contains_key(&window_label) {
            return Err(format!("session already exists for window {window_label}"));
        }

        let session_id = uuid::Uuid::new_v4().to_string();

        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|err| format!("failed to open pty: {err}"))?;

        let mut command = CommandBuilder::new(command_path.display().to_string());
        for arg in args {
            command.arg(arg);
        }
        command.env("TERM", "xterm-256color");
        command.env("COLORTERM", "truecolor");

        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|err| format!("failed to spawn mc: {err}"))?;

        let master = pair.master;
        let mut reader = master
            .try_clone_reader()
            .map_err(|err| format!("failed to clone pty reader: {err}"))?;
        let writer = master
            .take_writer()
            .map_err(|err| format!("failed to take pty writer: {err}"))?;

        let master = Arc::new(Mutex::new(master));
        let writer = Arc::new(Mutex::new(writer));
        let child = Arc::new(Mutex::new(child));

        let info = PtySessionInfo {
            session_id: session_id.clone(),
            window_label: window_label.clone(),
            mc_path: command_path.display().to_string(),
        };

        if let Some(app_for_reader) = app {
            let event_name = format!("pty-output-{window_label}");
            let title_label = window_label.clone();
            let exit_label = window_label.clone();
            let exit_app = app_for_reader.clone();
            let exit_child = child.clone();

            thread::spawn(move || {
                let mut buffer = [0u8; 8192];
                loop {
                    match reader.read(&mut buffer) {
                        Ok(0) => break,
                        Ok(count) => {
                            let chunk = String::from_utf8_lossy(&buffer[..count]).to_string();
                            if let Some(dir) = parse_mc_dir_from_chunk(&chunk) {
                                if let Some(window) =
                                    app_for_reader.get_webview_window(&title_label)
                                {
                                    let title = format_window_title(&dir);
                                    let _ = window.set_title(&title);
                                }
                            }
                            let _ = app_for_reader.emit(&event_name, chunk);
                        }
                        Err(_) => break,
                    }
                }

                if let Ok(mut child) = exit_child.lock() {
                    let _ = child.wait();
                }

                let _ = exit_app.emit(&format!("pty-exit-{exit_label}"), ());
                PtySessionRegistry::close_window_for_label(&exit_app, &exit_label);
            });
        }

        sessions.insert(
            window_label.clone(),
            PtySession {
                info: info.clone(),
                master,
                writer,
                child,
            },
        );

        Ok(info)
    }

    pub fn write_to_session(&self, window_label: &str, data: &str) -> Result<(), String> {
        let sessions = self.sessions.lock().map_err(|_| "session lock poisoned")?;
        let session = sessions
            .get(window_label)
            .ok_or_else(|| format!("no session for window {window_label}"))?;
        let mut writer = session
            .writer
            .lock()
            .map_err(|_| "writer lock poisoned")?;
        writer
            .write_all(data.as_bytes())
            .map_err(|err| format!("failed to write to pty: {err}"))?;
        writer
            .flush()
            .map_err(|err| format!("failed to flush pty: {err}"))?;
        Ok(())
    }

    pub fn resize_session(&self, window_label: &str, cols: u16, rows: u16) -> Result<(), String> {
        let sessions = self.sessions.lock().map_err(|_| "session lock poisoned")?;
        let session = sessions
            .get(window_label)
            .ok_or_else(|| format!("no session for window {window_label}"))?;
        let master = session
            .master
            .lock()
            .map_err(|_| "master lock poisoned")?;
        master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|err| format!("failed to resize pty: {err}"))
    }

    pub fn close_window_for_label(app: &AppHandle, window_label: &str) {
        if let Some(registry) = app.try_state::<PtySessionRegistry>() {
            registry.destroy_session(window_label);
        }

        if let Some(window) = app.get_webview_window(window_label) {
            let _ = window.close();
        }
    }

    pub fn destroy_session(&self, window_label: &str) -> bool {
        let mut sessions = match self.sessions.lock() {
            Ok(guard) => guard,
            Err(_) => return false,
        };

        let Some(session) = sessions.remove(window_label) else {
            return false;
        };

        if let Ok(mut child) = session.child.lock() {
            let _ = child.kill();
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve_mc::resolve_mc;

    #[test]
    fn registry_create_maps_window_label() {
        let registry = PtySessionRegistry::new();
        let mc_path = resolve_mc().expect("mc installed");
        let info = registry
            .create_session(None, "test-window".to_string(), 80, 24)
            .expect("create session");

        assert_eq!(info.window_label, "test-window");
        assert_eq!(info.mc_path, mc_path.display().to_string());
        assert!(registry.has_session("test-window"));
        assert_eq!(registry.session_count(), 1);
    }

    #[test]
    fn registry_destroy_removes_only_target_session() {
        let registry = PtySessionRegistry::new();

        registry
            .create_session_with_command(
                None,
                "window-a".to_string(),
                PathBuf::from("/bin/sleep"),
                vec!["60".to_string()],
                80,
                24,
            )
            .expect("create a");
        registry
            .create_session_with_command(
                None,
                "window-b".to_string(),
                PathBuf::from("/bin/sleep"),
                vec!["60".to_string()],
                80,
                24,
            )
            .expect("create b");
        assert_eq!(registry.session_count(), 2);

        assert!(registry.destroy_session("window-a"));
        assert_eq!(registry.session_count(), 1);
        assert!(!registry.has_session("window-a"));
        assert!(registry.has_session("window-b"));

        assert!(registry.destroy_session("window-b"));
        assert_eq!(registry.session_count(), 0);
    }

    #[test]
    fn dry_run_uses_resolved_mc_path() {
        let registry = PtySessionRegistry::new();
        let mc_path = resolve_mc().expect("mc installed");
        let message = registry
            .dry_run_for_window("dry-window")
            .expect("dry run");
        assert!(message.contains(&mc_path.display().to_string()));
        assert!(message.contains("dry-window"));
    }
}