// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod database;
mod error;

use database::Database;
use error::{Error, Result};
use log::debug;
use reqwest::multipart::Part;
use std::fs;
use tauri::Manager;
use uuid::Uuid;
use walkdir::{DirEntry, WalkDir};

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
    debug!("Saving selected directories {:?}", dirs);
    database.save_directories(&dirs)?;

    let files = dirs
        .into_iter()
        .map(get_files_from_dir)
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    let current_time = std::time::SystemTime::now();
    index_files(&files, &database)?;
    debug!("Indexing took {:?}", current_time.elapsed().unwrap());

    return Ok(());
}

fn index_files(files: &Vec<DirEntry>, database: &tauri::State<Database>) -> Result<()> {
    let mut descriptors = vec![];
    for file in files {
        let path = file
            .path()
            .to_str()
            .ok_or(Error::Generic("Could not get string from path".to_owned()))?
            .to_owned();
        descriptors.push(FileDesc {
            path,
            uuid: Uuid::new_v4(),
        });
    }

    database.index_files(descriptors)?;

    return Ok(());
}

pub struct FileDesc {
    path: String,
    uuid: Uuid,
}

fn get_files_from_dir(dir: &str) -> Result<Vec<DirEntry>> {
    return WalkDir::new(dir)
        .into_iter()
        .map(|res| res.map_err(|err| Error::Walkdir(err)))
        .collect::<Result<Vec<_>>>()
        .map(|vec| {
            vec.into_iter()
                .filter(|entry| entry.file_type().is_file())
                .collect::<Vec<_>>()
        });
}

#[tauri::command]
fn get_indexed_images(database: tauri::State<Database>) -> Result<Vec<String>> {
    debug!("Getting indexed files");
    return database.get_indexed_images();
}

#[tauri::command]
fn has_images_dirs(database: tauri::State<Database>) -> Result<bool> {
    debug!("Checking images dirs");
    return database.has_images_dirs();
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            send_image,
            save_images_dirs,
            has_images_dirs,
            get_indexed_images,
        ])
        .setup(|app| {
            env_logger::Builder::new()
                .filter_level(log::LevelFilter::Trace)
                .init();

            let path = app.path_resolver().app_data_dir().ok_or(Error::Runtime(
                "Could not get app data directory".to_owned(),
            ))?;
            app.manage(Database::init(path)?);
            update_scopes(app)?;
            return Ok(());
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn update_scopes(app: &tauri::App) -> Result<()> {
    let database = app.state::<Database>();
    let dirs = database.get_directories()?;

    for dir in dirs {
        debug!("Updating scope for {:?}", dir);
        app.asset_protocol_scope().allow_directory(dir, true)?;
    }

    return Ok(());
}
