use crate::database::Database;
use crate::error::Result;
use crate::image::{generate_thumbnail, ThumbnailParams};
use anyhow::{anyhow, Context};
use log::debug;
use std::path::Path;
use std::{fs, path::PathBuf};
use uuid::Uuid;

pub fn image_protocol_handler(
    database: &Database,
    app_data_dir: &Path,
    path: &str,
) -> Result<Vec<u8>> {
    let segments = path.split('/').collect::<Vec<_>>();
    let (id, thumbnail_params) = match segments[..] {
        [id, "original"] => Some((id.to_owned(), ThumbnailParams::default())),
        [id, size, mode] => Some((id.to_owned(), ThumbnailParams::from_str(size, mode)?)),
        _ => None,
    }
    .ok_or(anyhow!(
        "Could not parse request path {} to thumbnail file name",
        path
    ))?;
    let thumbnail_file_name = PathBuf::from(&id).join(thumbnail_params.to_string());

    debug!("thumbnail_file_name: {:?}", thumbnail_file_name);
    let thumbnails_dir = app_data_dir.join("thumbnails");
    let thumbnail_path = thumbnails_dir.join(thumbnail_file_name);
    debug!("thumbnail_path: {:?}", thumbnail_path);
    if !(fs::exists(&thumbnail_path).context(format!(
        "Failed when checking if file exists {:?}",
        &thumbnail_path
    ))?) {
        debug!(
            "Thumbnail path {:?} does not exist, generating new thumbnail",
            &thumbnail_path
        );
        let uuid = Uuid::parse_str(&id).context(format!("Could not parse uuid {}", id))?;
        let file_descriptor = database.get_file_by_id(&uuid)?;
        generate_thumbnail(&file_descriptor, &thumbnails_dir, &thumbnail_params);
    };
    let file = fs::read(&thumbnail_path).context(format!(
        "Failed when reading thumbnail {:?}",
        &thumbnail_path
    ))?;
    Ok(file)
}
