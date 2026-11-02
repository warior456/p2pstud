// src-tauri/src/main.rs
// Tauri application entry point.
// All heavy lifting lives in lib.rs; this file is kept minimal.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    p2pstud_lib::run();
}

