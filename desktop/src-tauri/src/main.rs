// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod database;
mod error;
mod image;

use crate::image::image_protocol::image_protocol_handler;
use base64ct::{Base64, Encoding};
use database::Database;
use error::{Error, Result};
use log::debug;
use reqwest::multipart::Part;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use tauri::{AppHandle, Manager};
use time::format_description::FormatItem;
use time::macros::format_description;
use time::OffsetDateTime;
use uuid::Uuid;
use walkdir::{DirEntry, WalkDir};

const DATE_TIME_FORMAT: &'static [FormatItem<'static>] = format_description!(
    "[year]-[month]-[day] [hour]:[minute]:[second] \"[offset_hour]:[offset_minute]\""
);

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

#[derive(Clone, serde::Serialize)]
struct FilesIndexed {
    total: usize,
}

#[derive(Clone, serde::Serialize)]
struct ThumbnailsGenerated {
    done: usize,
    total: usize,
    latest: String,
}

#[tauri::command]
async fn save_images_dirs(
    dirs: Vec<&str>,
    app_handle: AppHandle,
    database: tauri::State<'_, Database>,
) -> Result<()> {
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
    debug!("Start indexing");
    let descriptors = index_files(&files, &database)?;
    debug!("Indexing took {:?}", current_time.elapsed().unwrap());
    app_handle.emit_all(
        "files-indexed",
        FilesIndexed {
            total: descriptors.len(),
        },
    )?;

    generate_thumbnails(&descriptors, &app_handle)?;

    return Ok(());
}

fn generate_thumbnails(files: &Vec<FileDesc>, app_handle: &AppHandle) -> Result<()> {
    let thumbnails_dir = app_handle
        .path_resolver()
        .app_data_dir()
        .ok_or(error::Error::Generic(
            "Cannot get app data directory".to_owned(),
        ))?;
    let thumbnails_dir = thumbnails_dir.join("thumbnails");
    fs::create_dir_all(&thumbnails_dir).unwrap();

    let mut done: usize = 0;
    for file in files {
        debug!("Generating thumbnail for {}", &file.path);
        let thumbnail_paths = crate::image::generate_thumbnails(&file, &thumbnails_dir);
        let thumbnail_path = thumbnail_paths
            .get(0)
            .unwrap()
            .to_str()
            .unwrap_or("")
            .to_owned();
        done += 1;
        app_handle.emit_all(
            "thumbnails-generated",
            ThumbnailsGenerated {
                done,
                total: files.len(),
                latest: thumbnail_path,
            },
        )?;
    }

    return Ok(());
}

fn index_files(files: &Vec<DirEntry>, database: &tauri::State<Database>) -> Result<Vec<FileDesc>> {
    let mut descriptors = vec![];
    for file in files {
        let path = file
            .path()
            .to_str()
            .ok_or(Error::Generic("Could not get string from path".to_owned()))?
            .to_owned();

        let sha256 = hash(file.path())?;

        let date = get_date(&file, &path)?;
        descriptors.push(FileDesc {
            path,
            uuid: Uuid::new_v4(),
            date,
            sha256,
        });
    }
    debug!("Saving to db");
    database.index_files(&descriptors)?;

    return Ok(descriptors);
}

fn hash(path: &Path) -> Result<String> {
    let file = fs::read(path)?;
    let hash = Sha256::digest(&file);
    let encoded = Base64::encode_string(&hash);
    return Ok(encoded);
}

fn get_date(file: &DirEntry, path: &String) -> Result<OffsetDateTime> {
    return get_date_from_exif(path).or_else(|_| get_file_date(file));
}

fn get_file_date(file: &DirEntry) -> Result<OffsetDateTime> {
    let created = file.metadata()?.created()?;
    return Ok(created.into());
}

fn get_date_from_exif(path: &String) -> Result<OffsetDateTime> {
    let file = std::fs::File::open(path)?;
    let mut buf_reader = std::io::BufReader::new(&file);
    let exif_reader = exif::Reader::new();
    let exif = exif_reader.read_from_container(&mut buf_reader)?;

    let date = exif
        .get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY)
        .ok_or(Error::Generic("could not get date from image".to_owned()))?;
    let date = date.display_value().to_string();

    let offset = exif
        .get_field(exif::Tag::OffsetTimeOriginal, exif::In::PRIMARY)
        .ok_or(Error::Generic("could not get time offset".to_owned()))?;
    let offset = offset.display_value().to_string();

    let datetime = format!("{date} {offset}");
    let datetime = OffsetDateTime::parse(&datetime, DATE_TIME_FORMAT)?;

    return Ok(datetime);
}

pub struct FileDesc {
    path: String,
    uuid: Uuid,
    date: OffsetDateTime,
    sha256: String,
}

fn get_files_from_dir(dir: &str) -> Result<Vec<DirEntry>> {
    return WalkDir::new(dir)
        .into_iter()
        .map(|res| res.map_err(|err| Error::Walkdir(err)))
        .collect::<Result<Vec<_>>>()
        .map(|vec| {
            vec.into_iter()
                .filter(|entry| entry.file_type().is_file())
                .filter(|entry| {
                    let file_name = entry.file_name().to_str().unwrap();
                    return file_name.ends_with(".jpg") || file_name.ends_with(".jpeg");
                })
                .collect::<Vec<_>>()
        });
}

#[derive(Debug, Serialize)]
struct Image {
    id: Uuid,
    path: String,
    thumbnail_small: String,
    thumbnail_big: String,
}

#[tauri::command]
fn get_images(app_handle: AppHandle, database: tauri::State<Database>) -> Result<Vec<Image>> {
    debug!("Getting indexed files");
    let descriptors = database.get_indexed_images()?;

    let thumbnails_dir = app_handle
        .path_resolver()
        .app_data_dir()
        .ok_or(error::Error::Generic(
            "Cannot get app data directory".to_owned(),
        ))?;
    let thumbnails_dir = thumbnails_dir.join("thumbnails");

    let images = descriptors
        .into_iter()
        .map(|desc| {
            let thumbnail_small = thumbnails_dir
                .join(&desc.uuid.to_string())
                .join("512-cover")
                .to_str()
                .map(|str| str.to_owned())
                .unwrap_or(String::default());
            let thumbnail_big = thumbnails_dir
                .join(&desc.uuid.to_string())
                .join("1920-contain")
                .to_str()
                .map(|str| str.to_owned())
                .unwrap_or(String::default());
            return Image {
                id: desc.uuid,
                path: desc.path,
                thumbnail_small,
                thumbnail_big,
            };
        })
        .collect();

    return Ok(images);
}

#[tauri::command]
fn get_indexed_images_paged(
    page: usize,
    app_handle: AppHandle,
    database: tauri::State<Database>,
) -> Result<Vec<String>> {
    debug!("Getting indexed files");
    let descriptors = database.get_indexed_images_paged(page)?;

    let thumbnails_dir = app_handle
        .path_resolver()
        .app_data_dir()
        .ok_or(error::Error::Generic(
            "Cannot get app data directory".to_owned(),
        ))?;
    let thumbnails_dir = thumbnails_dir.join("thumbnails");

    let paths = descriptors
        .iter()
        .filter_map(|desc| {
            thumbnails_dir
                .join(&desc.uuid.to_string())
                .to_str()
                .map(|str| str.to_owned())
        })
        .collect();

    return Ok(paths);
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
            get_images,
            get_indexed_images_paged,
        ])
        .register_uri_scheme_protocol("image", image_protocol_handler)
        .setup(|app| {
            env_logger::Builder::new()
                .filter_level(log::LevelFilter::Trace)
                .init();

            let path = app.path_resolver().app_data_dir().ok_or(Error::Runtime(
                "Could not get app data directory".to_owned(),
            ))?;
            fs::create_dir_all(&path)?;

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
