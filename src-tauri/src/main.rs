// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::args().nth(1).as_deref() == Some("--verify-sessions") {
        midnight_commander_lib::verify_sessions_cli()
            .expect("session verification failed");
        return;
    }

    midnight_commander_lib::run();
}