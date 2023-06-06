// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn get_images() {
    println!("get_images");
    let dir = fs::read_dir("/home/climbingdev/dev/photo-store/images")
        .unwrap();
    dir.for_each(|x| {
        println!("{:?}", x);
    });
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![greet])
        .invoke_handler(tauri::generate_handler![get_images])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
