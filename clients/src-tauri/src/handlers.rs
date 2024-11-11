use std::fs;
use std::path::Path;

use base64ct::{Base64, Encoding};
use log::debug;
use rsa::RsaPublicKey;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager};
use time::format_description::FormatItem;
use time::macros::format_description;
use time::OffsetDateTime;
use uuid::Uuid;
use walkdir::{DirEntry, WalkDir};

use crate::database::Database;
use crate::error::{Error, Result};
use crate::files::{FileDescriptor, SyncStatus};
use crate::state::SyncedAppState;

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
pub(crate) fn save_images_dirs(
    dirs: Vec<&str>,
    app_handle: AppHandle,
    database: tauri::State<'_, Database>,
    app_state: tauri::State<'_, SyncedAppState>,
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

    let state = app_state.read();
    let auth_ctx = state
        .auth_ctx
        .ok_or(Error::Generic("User is not authenticated".to_owned()))?;

    let public_key = auth_ctx.get_public_key();

    let current_time = std::time::SystemTime::now();
    debug!("Start indexing");
    let descriptors = index_files(&files, &public_key, &database)?;
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

fn generate_thumbnails(files: &Vec<FileDescriptor>, app_handle: &AppHandle) -> Result<()> {
    debug!("Start generating thumbnails");
    let thumbnails_dir = app_handle
        .path()
        .app_data_dir()
        .ok()
        .ok_or(Error::Generic("Cannot get app data directory".to_owned()))?;
    let thumbnails_dir = thumbnails_dir.join("thumbnails");
    debug!("Thumbnails directory {:?}", thumbnails_dir);
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

    Ok(())
}

fn index_files(
    files: &Vec<DirEntry>,
    public_key: &RsaPublicKey,
    database: &tauri::State<Database>,
) -> Result<Vec<FileDescriptor>> {
    let mut descriptors = vec![];
    for file in files {
        let path = file
            .path()
            .to_str()
            .ok_or(Error::Generic("Could not get string from path".to_owned()))?
            .to_owned();

        let sha256 = hash(file.path())?;

        let encryption_key = crypto::generate_encoded_encryption_key(public_key);

        let date = get_date(file, &path)?;
        descriptors.push(FileDescriptor {
            path,
            uuid: Uuid::new_v4(),
            date,
            sha256,
            key: encryption_key,
            status: SyncStatus::New,
        });
    }
    debug!("Saving to db");
    database.index_files(&descriptors)?;

    Ok(descriptors)
}

fn hash(path: &Path) -> Result<String> {
    let file = fs::read(path)?;
    let hash = Sha256::digest(file);
    let encoded = Base64::encode_string(&hash);
    Ok(encoded)
}

fn get_date(file: &DirEntry, path: &String) -> Result<OffsetDateTime> {
    get_date_from_exif(path).or_else(|_| get_file_date(file))
}

fn get_file_date(file: &DirEntry) -> Result<OffsetDateTime> {
    let created = file.metadata()?.created()?;
    Ok(created.into())
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

    Ok(datetime)
}

fn get_files_from_dir(dir: &str) -> Result<Vec<DirEntry>> {
    WalkDir::new(dir)
        .into_iter()
        .map(|res| res.map_err(Error::Walkdir))
        .collect::<Result<Vec<_>>>()
        .map(|vec| {
            vec.into_iter()
                .filter(|entry| entry.file_type().is_file())
                .filter(|entry| {
                    let file_name = entry.file_name().to_str().unwrap();
                    file_name.ends_with(".jpg") || file_name.ends_with(".jpeg")
                })
                .collect::<Vec<_>>()
        })
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

    Ok(images)
}

#[tauri::command]
pub(crate) fn has_images_dirs(database: tauri::State<Database>) -> Result<bool> {
    debug!("Checking images dirs");
    database.has_images_dirs()
}
