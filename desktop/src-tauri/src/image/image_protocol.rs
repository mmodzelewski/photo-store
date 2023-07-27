use std::{
    fs,
    io::{BufWriter, Read},
    path::PathBuf,
};

use image::{
    codecs::jpeg::{JpegDecoder, JpegEncoder},
    DynamicImage, ImageDecoder,
};
use log::debug;
use tauri::{
    async_runtime,
    http::{Request, Response, ResponseBuilder},
    AppHandle, Manager,
};
use url::Url;

use crate::{database::Database, error::Error};

pub fn image_protocol_handler(
    app: &AppHandle,
    request: &Request,
) -> Result<Response, Box<dyn std::error::Error>> {
    let url = Url::parse(request.uri())?;
    let wins = app.windows();
    let win = wins.values().take(1).collect::<Vec<_>>()[0];
    let win_uri = win.url();
    debug!("win url {win_uri}");

    let path = url.path()[1..].to_owned();
    let segments = path.split('/').collect::<Vec<_>>();

    let thumbnail_res = match segments[..] {
        [id, "original"] => Some((id, PathBuf::from(id).join("1920-contain"))),
        [id, size, mode] => Some((id, PathBuf::from(id).join(format!("{}-{}", size, mode)))),
        _ => None,
    };
    debug!("thumbnail_file_name: {:?}", thumbnail_res);

    let app_data_dir = app.path_resolver().app_data_dir().ok_or(Error::Runtime(
        "Could not get app data directory".to_owned(),
    ))?;

    if let Some((id, thumbnail_file_name)) = thumbnail_res {
        let thumbnails_folder = app_data_dir.join("thumbnails");
        let thumbnail_path = thumbnails_folder.join(thumbnail_file_name);
        debug!("thumbnail_path: {:?}", thumbnail_path);

        if thumbnail_path.exists() {
            let file = fs::read(thumbnail_path)?;
            return ResponseBuilder::new()
                .status(202)
                .header("Access-Control-Allow-Origin", "*")
                .body(file);
        } else {
            debug!("Generating thumbnail");
            let database = app.state::<Database>();
            let image = database.get_image(id)?;
            // debug!("Image loaded: {:?}", image);
            // debug!("Thumbnail folder: {:?}", thumbnails_folder);

            //let start_time = std::time::Instant::now();
            //let thumbnails = crate::image::generate_thumbnails(&image, &thumbnails_folder);
            //debug!("Thumbnail 1 generated in {:?}", start_time.elapsed());
            //          let thumbnail = &thumbnails[0];

            // debug!("Thumbnail generated: {:?}", thumbnail);
            let start_time = std::time::Instant::now();
            let file = fs::read(image.path)?;

            let mut decoder = JpegDecoder::new(&file[..])?;
            decoder.scale(512, 512)?;

            // let image = image::load_from_memory(&file)?;
            //let image = image.thumbnail(512, 512);
            let image = DynamicImage::from_decoder(decoder).unwrap();

            // create DynamicImage from buffer

            // write an image to a buffer
            let mut output = Vec::new();
            let mut encoder = JpegEncoder::new_with_quality(&mut output, 75);
            encoder.encode_image(&image)?;
            debug!("Thumbnail 2 generated in {:?}", start_time.elapsed());
            let buf = output;

            let window_url = win_uri;
            let window_origin = if window_url.scheme() == "data" {
                "null".into()
            } else if cfg!(windows)
                && window_url.scheme() != "http"
                && window_url.scheme() != "https"
            {
                format!("https://{}.localhost", window_url.scheme())
            } else {
                format!(
                    "{}://{}{}",
                    window_url.scheme(),
                    window_url.host().unwrap(),
                    window_url
                        .port()
                        .map(|p| format!(":{p}"))
                        .unwrap_or_default()
                )
            };

            debug!("window_origin: {:?}", window_origin);
            return ResponseBuilder::new()
                .header("Access-Control-Allow-Origin", "null")
                .mimetype("image/jpeg")
                .status(201)
                .body(buf);
        }
    } else {
        return ResponseBuilder::new()
            .status(400)
            .body("Invalid image URL".into());
    }
}
