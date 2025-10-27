use crate::auth::AuthCtx;
use crate::database::Database;
use crate::error::Result;
use crate::files::{File, Metadata};
use crate::http::HttpClient;
use crate::image::{generate_thumbnail, ThumbnailParams};
use anyhow::{anyhow, Context};
use crypto::{decode_encryption_key, decrypt_data};
use log::debug;
use std::path::Path;
use std::{fs, path::PathBuf};
use uuid::Uuid;

pub async fn image_protocol_handler(
    database: &Database,
    http_client: &HttpClient,
    auth_ctx: AuthCtx,
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
    if !fs::exists(&thumbnail_path).context(format!(
        "Failed when checking if file exists {:?}",
        &thumbnail_path
    ))? {
        debug!(
            "Thumbnail path {:?} does not exist, generating new thumbnail",
            &thumbnail_path
        );
        let uuid = Uuid::parse_str(&id).context(format!("Could not parse uuid {}", id))?;
        let file = database.get_file_by_id(&uuid)?;
        match file {
            File::Local { .. } => {
                let desc = file.try_into()?;
                generate_thumbnail(&desc, &thumbnails_dir, &thumbnail_params);
            }
            File::Remote { metadata } => {
                debug!("Getting remote file thumbnail");
                let img_path = thumbnails_dir.join(&id);
                fs::create_dir_all(&img_path).context("Could not create dirs")?;
                get_thumbnail_from_web(
                    http_client,
                    auth_ctx,
                    &thumbnail_params,
                    &thumbnail_path,
                    &metadata,
                )
                .await?;
            }
        }
    };
    let file = fs::read(&thumbnail_path).context(format!(
        "Failed when reading thumbnail {:?}",
        &thumbnail_path
    ))?;
    Ok(file)
}

async fn get_thumbnail_from_web(
    http_client: &HttpClient,
    auth_ctx: AuthCtx,
    thumbnail_params: &ThumbnailParams,
    output_dir: &PathBuf,
    metadata: &Metadata,
) -> Result<()> {
    let client = http_client.client();
    let data_response = client
        .get(format!(
            "{}/files/{}/data",
            http_client.url(),
            metadata.uuid
        ))
        .query(&[("variant", thumbnail_params.to_string())])
        .header("Authorization", auth_ctx.get_auth_token())
        .send()
        .await
        .context("Failed to fetch file data")?;

    let encrypted_data = data_response
        .bytes()
        .await
        .context("Failed to read file data")?;

    let encryption_key = decode_encryption_key(&metadata.key, auth_ctx.decrypt())
        .context(format!("Failed to decrypt key for file {}", metadata.uuid))?;
    let decrypted_data = decrypt_data(metadata.uuid, &encryption_key, encrypted_data).unwrap();
    fs::write(output_dir, decrypted_data).context(format!(
        "Failed to save file data, uuid: {}, path {:?}",
        metadata.uuid, &output_dir
    ))?;
    Ok(())
}
