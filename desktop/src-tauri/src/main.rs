// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{fs, sync::Mutex};

use reqwest::multipart::Part;
use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};
use tauri::{App, Manager};

struct Database {
    connection: Mutex<Connection>,
}

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

#[tauri::command]
fn save_images_dirs(dirs: Vec<&str>, database: tauri::State<Database>) {
    let mut conn = database.connection.lock().unwrap();

    let tx = conn.transaction().unwrap();
    {
        let mut stmt = tx
            .prepare("INSERT INTO directory (path) VALUES (?1)")
            .unwrap();
        dirs.iter().for_each(|dir| {
            stmt.execute([dir]).unwrap();
        });
    }
    tx.commit().unwrap();
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![send_image, save_images_dirs])
        .setup(|app| {
            init_db(&app);
            return Ok(());
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn init_db(app: &App) {
    let path = app.path_resolver().app_data_dir().unwrap();

    let migrations = Migrations::new(vec![M::up(
        "CREATE TABLE directory (
            id INTEGER PRIMARY KEY,
            path TEXT NOT NULL UNIQUE ON CONFLICT IGNORE
        );",
    )]);
    let mut conn = Connection::open(path.join("data.db3")).unwrap();
    // conn.pragma_update(None, "journal_mode", &"WAL").unwrap(); // verify
    migrations.to_latest(&mut conn).unwrap();

    app.manage(Database {
        connection: Mutex::new(conn),
    });
}
