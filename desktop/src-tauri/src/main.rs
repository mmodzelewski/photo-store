// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod database;
mod error;

use database::Database;
use error::{Error, Result};
use fast_image_resize as fr;
use fr::FilterType;
use image::codecs::jpeg::JpegEncoder;
use image::io::Reader as ImageReader;
use image::{ColorType, ImageEncoder};
use log::debug;
use reqwest::multipart::Part;
use std::{
    fs::{self, File},
    io::BufWriter,
    num::NonZeroU32,
    path::PathBuf,
};
use tauri::{AppHandle, Manager};
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
fn save_images_dirs(
    dirs: Vec<&str>,
    app_handle: AppHandle,
    database: tauri::State<Database>,
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
    let descriptors = index_files(&files, &database)?;
    debug!("Indexing took {:?}", current_time.elapsed().unwrap());

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

    for file in files {
        debug!("Generating thumbnail for {}", &file.path);
        generate_thumbnail(&file, &thumbnails_dir);
    }

    return Ok(());
}

fn generate_thumbnail(file_desc: &FileDesc, folder_path: &PathBuf) {
    let file = ImageReader::open(&file_desc.path).unwrap();
    let img = file.decode().unwrap();

    let width = NonZeroU32::new(img.width()).unwrap();
    let height = NonZeroU32::new(img.height()).unwrap();
    let src_image =
        fr::Image::from_vec_u8(width, height, img.to_rgb8().into_raw(), fr::PixelType::U8x3)
            .unwrap();
    let mut src_view = src_image.view();

    let dst_width = NonZeroU32::new(512).unwrap();
    let dst_height = NonZeroU32::new(512).unwrap();
    src_view.set_crop_box_to_fit_dst_size(dst_width, dst_height, None);
    let mut dst_image = fr::Image::new(dst_width, dst_height, src_view.pixel_type());

    let mut dst_view = dst_image.view_mut();

    let mut resizer = fr::Resizer::new(fr::ResizeAlg::Convolution(FilterType::Lanczos3));
    resizer.resize(&src_view, &mut dst_view).unwrap();

    let uuid = &file_desc.uuid;
    let thumbnail_path = folder_path.join(uuid.to_string());
    let thumbnail_file = File::create(thumbnail_path).unwrap();
    let mut result_buf = BufWriter::new(thumbnail_file);

    JpegEncoder::new(&mut result_buf)
        .write_image(
            dst_image.buffer(),
            dst_width.get(),
            dst_height.get(),
            ColorType::Rgb8,
        )
        .unwrap();
}

fn index_files(files: &Vec<DirEntry>, database: &tauri::State<Database>) -> Result<Vec<FileDesc>> {
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

    database.index_files(&descriptors)?;

    return Ok(descriptors);
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
                .filter(|entry| entry.file_name().to_str().unwrap().ends_with(".jpg"))
                .collect::<Vec<_>>()
        });
}

#[tauri::command]
fn get_indexed_images(
    app_handle: AppHandle,
    database: tauri::State<Database>,
) -> Result<Vec<String>> {
    debug!("Getting indexed files");
    let descriptors = database.get_indexed_images()?;

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
            get_indexed_images,
            get_indexed_images_paged,
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
