// Prevent an additional console window on Windows in release mode
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    orpheus_tauri_lib::run()
}
