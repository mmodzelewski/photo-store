use std::{fs, path::PathBuf};

use log::debug;
use tauri::{
    http::{Request, Response},
    AppHandle, Manager,
};

use crate::error::Error;

pub fn image_protocol_handler(app: &AppHandle, request: Request<Vec<u8>>) -> Response<Vec<u8>> {
    let url = request.uri();
    let path = url.path()[1..].to_owned();
    let segments = path.split('/').collect::<Vec<_>>();

    let thumbnail_file_name = match segments[..] {
        [id, "original"] => Some(PathBuf::from(id).join("1920-contain")),
        [id, size, mode] => Some(PathBuf::from(id).join(format!("{}-{}", size, mode))),
        _ => None,
    };
    debug!("thumbnail_file_name: {:?}", thumbnail_file_name);

    let app_data_dir = app
        .path()
        .app_data_dir()
        .ok()
        .ok_or(Error::Runtime(
            "Could not get app data directory".to_owned(),
        ))
        .unwrap();

    if let Some(thumbnail_file_name) = thumbnail_file_name {
        let thumbnail_path = app_data_dir.join("thumbnails").join(thumbnail_file_name);
        debug!("thumbnail_path: {:?}", thumbnail_path);
        let file = fs::read(thumbnail_path).unwrap();
        return Response::builder().status(200).body(file).unwrap();
    } else {
        return Response::builder()
            .status(400)
            .body("Invalid image URL".into())
            .unwrap();
    }
}
