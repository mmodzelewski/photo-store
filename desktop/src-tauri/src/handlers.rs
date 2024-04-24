use crate::database::Database;
use crate::error::{Error, Result};
use crate::http::{self, auth};
use base64ct::{Base64, Encoding};
use dtos::auth::LoginRequest;
use dtos::file::{FileMetadata, FilesUploadRequest};
use log::debug;
use reqwest::multipart::Part;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::{fs, sync::Mutex};
use tauri::{AppHandle, Manager};
use time::format_description::FormatItem;
use time::macros::format_description;
use time::OffsetDateTime;
use uuid::Uuid;
use walkdir::{DirEntry, WalkDir};

const DATE_TIME_FORMAT: &[FormatItem<'static>] = format_description!(
    "[year]-[month]-[day] [hour]:[minute]:[second] \"[offset_hour]:[offset_minute]\""
);

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
pub(crate) async fn save_images_dirs(
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
    app_handle.emit(
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
        .path()
        .app_data_dir()
        .ok()
        .ok_or(Error::Generic("Cannot get app data directory".to_owned()))?;
    let thumbnails_dir = thumbnails_dir.join("thumbnails");
    fs::create_dir_all(&thumbnails_dir).unwrap();

    for (done, file) in files.iter().enumerate() {
        debug!("Generating thumbnail for {}", &file.path);
        let thumbnail_paths = crate::image::generate_thumbnails(file, &thumbnails_dir);
        let thumbnail_path = thumbnail_paths
            .get(0)
            .unwrap()
            .to_str()
            .unwrap_or("")
            .to_owned();
        app_handle.emit(
            "thumbnails-generated",
            ThumbnailsGenerated {
                done: done + 1,
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

        let date = get_date(file, &path)?;
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
    let hash = Sha256::digest(file);
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

pub(crate) struct FileDesc {
    pub path: String,
    pub uuid: Uuid,
    pub date: OffsetDateTime,
    pub sha256: String,
}

fn get_files_from_dir(dir: &str) -> Result<Vec<DirEntry>> {
    return WalkDir::new(dir)
        .into_iter()
        .map(|res| res.map_err(Error::Walkdir))
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
pub(crate) struct Image {
    id: Uuid,
    path: String,
}

#[tauri::command]
pub(crate) fn get_images(database: tauri::State<Database>) -> Result<Vec<Image>> {
    debug!("Getting indexed files");
    let descriptors = database.get_indexed_images()?;
    let images = descriptors
        .into_iter()
        .map(|desc| Image {
            id: desc.uuid,
            path: desc.path,
        })
        .collect();

    return Ok(images);
}

#[tauri::command]
pub(crate) async fn sync_images(
    database: tauri::State<'_, Database>,
    app_state: tauri::State<'_, AppState>,
) -> Result<()> {
    debug!("sync_images called");

    let user_data = { app_state.user_data.lock().unwrap().clone() };
    let user_data = user_data.ok_or(Error::Generic("User is not logged in".to_owned()))?;

    let descriptors = database.get_indexed_images()?;
    let image_metadata = descriptors
        .iter()
        .map(|desc| FileMetadata {
            path: desc.path.to_owned(),
            uuid: desc.uuid,
            date: desc.date,
            sha256: desc.sha256.to_owned(),
        })
        .collect();

    let body = FilesUploadRequest {
        user_id: user_data.user_id,
        files: image_metadata,
    };
    debug!("Sending metadata: {:?}", body);

    let http_client = { app_state.http_client.lock().unwrap().clone() };
    let client = http_client.client;

    let response = client
        .post(format!("{}/files/metadata", http_client.url))
        .header("Content-Type", "application/json")
        .header("Authorization", &user_data.auth_token)
        .body(serde_json::to_string(&body).unwrap())
        .send()
        .await
        .unwrap();
    debug!("Response: {:?}", response);

    debug!("Sending files");
    for desc in descriptors {
        let file = fs::read(&desc.path).unwrap();

        let form = reqwest::multipart::Form::new().part(
            "file",
            Part::bytes(file)
                .file_name(desc.path.to_owned())
                .mime_str("image/jpeg")
                .unwrap(),
        );
        println!("Sending file: {:?}", &desc.path);
        let res = client
            .post(format!("{}/files/{}/data", http_client.url, desc.uuid))
            .header("Authorization", &user_data.auth_token)
            .multipart(form)
            .send()
            .await;
        println!("{:?}", res);
    }
    return Ok(());
}

#[tauri::command]
pub(crate) fn has_images_dirs(database: tauri::State<Database>) -> Result<bool> {
    debug!("Checking images dirs");
    return database.has_images_dirs();
}

#[tauri::command]
pub(crate) async fn login(
    username: String,
    password: String,
    state: tauri::State<'_, AppState>,
) -> Result<()> {
    debug!("Logging in");

    let body = LoginRequest { username, password };
    let http_client = { state.http_client.lock().unwrap().clone() };
    let response = auth::login(http_client, &body).await?;

    *state.user_data.lock().unwrap() = Some(UserData {
        auth_token: response.auth_token,
        user_id: response.user_id,
    });
    return Ok(());
}

pub(crate) struct AppState {
    pub user_data: Mutex<Option<UserData>>,
    pub http_client: Mutex<http::HttpClient>,
}

#[derive(Clone)]
pub(crate) struct UserData {
    pub user_id: Uuid,
    pub auth_token: String,
}
