// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;

use reqwest::multipart::Part;

#[tauri::command]
async fn send_image() {
    println!("send_image called");
    let mut dir = fs::read_dir("/home/climbingdev/Pictures/images").unwrap();
    let path = dir.next().unwrap().unwrap().path();
    let file = fs::read(path).unwrap();

     let form = reqwest::multipart::Form::new().part(
         "file",
         Part::bytes(file)
             .file_name("test.jpg")
             .mime_str("image/jpeg")
             .unwrap(),
     );
     let client = reqwest::Client::new();
    let res = client
        .post("http://localhost:3000/upload")
        .multipart(form)
        .send()
        .await;
    println!("{:?}", res);
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![send_image])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
