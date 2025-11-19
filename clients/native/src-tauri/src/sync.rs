use crate::auth::AuthCtx;
use crate::error::Result;
use crate::files::FileStatus::{Synced, UploadInProgress};
use crate::files::{FileDescriptor, FileDescriptorWithDecodedKey, FileStatus};
use crate::image::{ThumbnailParams, generate_thumbnail};
use crate::state::{SyncedAppState, User};
use crate::{database::Database, http::HttpClient};
use anyhow::{Context, anyhow};
use crypto::{decode_encryption_key, encrypt_data};
use dtos::file::{FileMetadata, FilesUploadRequest};
use log::{debug, error, warn};
use reqwest::header::HeaderMap;
use reqwest::multipart::Part;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager, State};
use time::OffsetDateTime;

#[tauri::command]
pub(crate) async fn sync_images(
    app_handle: AppHandle,
    database: State<'_, Database>,
    app_state: State<'_, SyncedAppState>,
    http_client: State<'_, HttpClient>,
) -> Result<()> {
    debug!("sync_images called");

    let state = app_state.read();
    let (user, auth_ctx) = state.get_authenticated_user()?;

    download_new_files(&app_handle, &database, &http_client, &auth_ctx).await?;
    upload_new_files(&app_handle, &database, &http_client, user, &auth_ctx).await?;
    Ok(())
}

async fn download_new_files(
    app_handle: &AppHandle,
    database: &State<'_, Database>,
    http_client: &State<'_, HttpClient>,
    auth_ctx: &AuthCtx,
) -> Result<()> {
    let now = OffsetDateTime::now_utc();
    let sync_time = database.get_last_sync_time()?;

    let client = http_client.client();
    let response = client
        .get(format!("{}/files/metadata", http_client.url()))
        .header("Authorization", auth_ctx.get_auth_token())
        .query(&[("from", &sync_time.map(|t| t.unix_timestamp()))])
        .send()
        .await
        .context("Failed to fetch files from backend")?;

    let remote_files: Vec<FileMetadata> = response
        .json()
        .await
        .context("Failed to parse remote files response")?;
    debug!("Fetched metadata for {} files", remote_files.len());

    let dirs = database.get_directories()?;
    // todo: take proper directory for downloads
    let output_dir = dirs.first().ok_or(anyhow!("No directories selected"))?;
    let mut new_files_downloaded = false;
    for remote_file in remote_files.iter() {
        if !database.file_exists(&remote_file.uuid)? {
            debug!(
                "Fetching new file: {} {:?}",
                remote_file.uuid, remote_file.path
            );
            let path = format!("{}/{}.jpg", output_dir, remote_file.uuid);
            let descriptor = FileDescriptor {
                path: path.to_owned(),
                uuid: remote_file.uuid,
                date: remote_file.date,
                sha256: remote_file.sha256.to_owned(),
                key: remote_file.key.to_owned(),
                status: Synced,
            };
            let descriptors = vec![descriptor];
            database.index_files(&descriptors, true)?;

            new_files_downloaded = true;
        }
    }
    if new_files_downloaded {
        app_handle
            .emit("index-updated", ())
            .context("Couldn't emit index-updated")?;
    }
    database.update_last_sync_time(&now)?;
    Ok(())
}

async fn upload_new_files(
    app_handle: &AppHandle,
    database: &State<'_, Database>,
    http_client: &State<'_, HttpClient>,
    user: User,
    auth_ctx: &AuthCtx,
) -> Result<()> {
    let client = http_client.client();
    let files_to_upload = get_files_to_upload(database, auth_ctx)?;
    if files_to_upload.is_empty() {
        debug!("No files to upload");
        return Ok(());
    }

    let files_metadata = files_to_upload
        .iter()
        .map(|desc| desc.descriptor().into())
        .collect();

    let body = FilesUploadRequest {
        user_id: user.id,
        files: files_metadata,
    };
    debug!("Sending metadata: {:?}", body);

    let response = client
        .post(format!("{}/files/metadata", http_client.url()))
        .header("Content-Type", "application/json")
        .header("Authorization", auth_ctx.get_auth_token())
        .body(serde_json::to_string(&body).unwrap())
        .send()
        .await
        .unwrap();
    debug!("Response: {:?}", response);

    debug!("Sending files");
    for descriptor_with_key in files_to_upload {
        let descriptor = descriptor_with_key.descriptor();
        let key = descriptor_with_key.key();

        database.update_file_status(&descriptor.uuid, UploadInProgress)?;

        let file = fs::read(&descriptor.path).unwrap();
        let (encrypted_data, encrypted_data_hash) = encrypt_data(descriptor, key, file.into())
            .with_context(|| format!("Could not encrypt file {:?}", descriptor.path))?;

        let thumbnail_paths = prepare_thumbnails(app_handle, descriptor)?;

        let mut headers = HeaderMap::new();
        headers.insert("sha256_checksum", encrypted_data_hash.parse().unwrap());
        let mut form = reqwest::multipart::Form::new().part(
            "original",
            Part::bytes(encrypted_data)
                .headers(headers)
                .mime_str("image/jpeg")
                .unwrap(),
        );

        for (thumbnail_name, thumbnail_path) in thumbnail_paths.into_iter() {
            let thumbnail_data = fs::read(&thumbnail_path)
                .context(format!("Couldn't read thumbnail {:?}", &thumbnail_path))?;
            let (encrypted_thumbnail, encrypted_thumbnail_hash) =
                encrypt_data(descriptor, key, thumbnail_data.into())
                    .context(format!("Failed to encrypt thumbnail {:?}", &thumbnail_path))?;

            let mut headers = HeaderMap::new();
            headers.insert("sha256_checksum", encrypted_thumbnail_hash.parse().unwrap());
            form = form.part(
                thumbnail_name.clone(),
                Part::bytes(encrypted_thumbnail)
                    .headers(headers)
                    .mime_str("image/jpeg")
                    .unwrap(),
            );
        }

        debug!("Sending file: {:?}", &descriptor.path);
        let upload_response = client
            .post(format!(
                "{}/files/{}/data",
                http_client.url(),
                descriptor.uuid
            ))
            .header("Authorization", auth_ctx.get_auth_token())
            .multipart(form)
            .send()
            .await;

        match upload_response {
            Ok(response) if response.status().is_success() => {
                database.update_file_status(&descriptor.uuid, Synced)?;
                debug!("File uploaded successfully: {:?}", &descriptor.path);
            }
            Ok(response) => {
                warn!(
                    "Failed to upload file: {:?}, status: {}",
                    &descriptor.path,
                    response.status()
                );
                debug!("{:?}", response);
            }
            Err(e) => {
                error!("Error uploading file: {:?}, error: {}", &descriptor.path, e);
            }
        }
    }
    Ok(())
}

fn prepare_thumbnails(
    app_handle: &AppHandle,
    descriptor: &FileDescriptor,
) -> Result<HashMap<String, PathBuf>> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .context("Could not get app data directory")?;
    let thumbnails_dir = app_data_dir.join("thumbnails");

    let small_params = ThumbnailParams::small_cover();
    let small = prepare_thumbnail(descriptor, &thumbnails_dir, &small_params)?;

    let big_params = ThumbnailParams::big_contain();
    let big = prepare_thumbnail(descriptor, &thumbnails_dir, &big_params)?;

    let thumbnails = HashMap::from([
        (format!("thumbnail-{}", small_params), small),
        (format!("thumbnail-{}", big_params), big),
    ]);
    Ok(thumbnails)
}

fn prepare_thumbnail(
    descriptor: &FileDescriptor,
    thumbnails_dir: &Path,
    params: &ThumbnailParams,
) -> Result<PathBuf> {
    let thumbnail = thumbnails_dir
        .join(descriptor.uuid.to_string())
        .join(params.to_string());
    if !fs::exists(&thumbnail).context(format!(
        "Failed when checking if file exists {:?}",
        &thumbnail
    ))? {
        generate_thumbnail(descriptor, thumbnails_dir, params);
    }
    Ok(thumbnail)
}

fn get_files_to_upload(
    database: &State<Database>,
    auth_ctx: &AuthCtx,
) -> Result<Vec<FileDescriptorWithDecodedKey>> {
    Ok(database
        .find_files_by_sync_status(FileStatus::New)?
        .into_iter()
        .map(|desc| {
            let key = decode_encryption_key(&desc.key, auth_ctx.decrypt())
                .context(format!(
                    "Failed to decode encryption key for: {}",
                    &desc.uuid
                ))
                .unwrap();
            FileDescriptorWithDecodedKey::new(desc, key)
        })
        .collect())
}
