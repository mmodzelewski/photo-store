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

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    DB(#[from] rusqlite::Error),
    #[error(transparent)]
    DBMigrations(#[from] rusqlite_migration::Error),
    #[error("{0}")]
    Generic(String),
    #[error("Runtime error: {0}")]
    Runtime(String),
}

type Result<T> = std::result::Result<T, Error>;

impl serde::Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
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
fn save_images_dirs(dirs: Vec<&str>, database: tauri::State<Database>) -> Result<()> {
    let mut conn = database
        .connection
        .lock()
        .map_err(|err| Error::Generic(err.to_string()))?;

    let tx = conn.transaction()?;
    {
        let mut stmt = tx.prepare("INSERT INTO directory (path) VALUES (?1)")?;
        dirs.iter()
            .map(|dir| stmt.execute([dir]))
            .collect::<std::result::Result<Vec<_>, _>>()?;
    }
    tx.commit()?;
    return Ok(());
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![send_image, save_images_dirs])
        .setup(|app| {
            init_db(&app)?;
            return Ok(());
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn init_db(app: &App) -> Result<()> {
    let path = app.path_resolver().app_data_dir().ok_or(Error::Runtime(
        "Could not get app data directory".to_owned(),
    ))?;

    let migrations = Migrations::new(vec![M::up(
        "CREATE TABLE directory (
            id INTEGER PRIMARY KEY,
            path TEXT NOT NULL UNIQUE ON CONFLICT IGNORE
        );",
    )]);
    let mut conn = Connection::open(path.join("data.db3"))?;
    // conn.pragma_update(None, "journal_mode", &"WAL").unwrap(); // verify
    migrations.to_latest(&mut conn)?;

    app.manage(Database {
        connection: Mutex::new(conn),
    });
    return Ok(());
}
