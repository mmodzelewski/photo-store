use crate::auth::AuthCtx;
use crate::error::Result;
use crate::files::{FileDescriptorWithDecodedKey, SyncStatus};
use crate::state::{SyncedAppState, User};
use crate::{database::Database, http::HttpClient};
use anyhow::{anyhow, Context};
use crypto::{decode_encryption_key, decrypt_data, encrypt_data};
use dtos::file::{FileMetadata, FilesUploadRequest};
use log::{debug, error, warn};
use reqwest::multipart::Part;
use std::collections::HashSet;
use std::fs;
use tauri::State;

#[tauri::command]
pub(crate) async fn sync_images(
    database: State<'_, Database>,
    app_state: State<'_, SyncedAppState>,
    http_client: State<'_, HttpClient>,
) -> Result<()> {
    debug!("sync_images called");

    let state = app_state.read();
    let user = state.user.context("User is not logged in")?;
    let auth_ctx = state.auth_ctx.context("User is not authenticated")?;

    download_new_files(&database, &http_client, &auth_ctx).await?;
    upload_new_files(&database, &http_client, user, &auth_ctx).await?;
    Ok(())
}

async fn download_new_files(
    database: &State<'_, Database>,
    http_client: &State<'_, HttpClient>,
    auth_ctx: &AuthCtx,
) -> Result<()> {
    let client = http_client.client();
    let response = client
        .get(format!("{}/files/metadata", http_client.url()))
        .header("Authorization", auth_ctx.get_auth_token())
        .send()
        .await
        .context("Failed to fetch files from backend")?;

    let remote_files: Vec<FileMetadata> = response
        .json()
        .await
        .context("Failed to parse remote files response")?;

    let all_local_files = database.get_indexed_images()?;
    let local_file_uuids_set = all_local_files
        .iter()
        .map(|desc| desc.uuid)
        .collect::<HashSet<_>>();

    let dirs = database.get_directories()?;
    // todo: take proper directory for downloads
    let output_dir = dirs.first().ok_or(anyhow!("No directories selected"))?;
    for remote_file in remote_files.iter() {
        if !local_file_uuids_set.contains(&remote_file.uuid) {
            // todo: save files to db
            debug!(
                "Fetching new file: {} {:?}",
                remote_file.uuid, remote_file.path
            );

            let data_response = client
                .get(format!(
                    "{}/files/{}/data",
                    http_client.url(),
                    remote_file.uuid
                ))
                .header("Authorization", auth_ctx.get_auth_token())
                .send()
                .await
                .context("Failed to fetch file data")?;

            let encrypted_data = data_response
                .bytes()
                .await
                .context("Failed to read file data")?;

            let encryption_key = decode_encryption_key(&remote_file.key, auth_ctx.decrypt())
                .context(format!(
                    "Failed to decrypt key for file {}",
                    remote_file.uuid
                ))?;
            let decrypted_data =
                decrypt_data(remote_file.uuid, &encryption_key, encrypted_data).unwrap();
            fs::write(
                format!("{}/{}.jpg", output_dir, remote_file.uuid),
                decrypted_data,
            )
            .unwrap();
        }
    }
    Ok(())
}

async fn upload_new_files(
    database: &State<'_, Database>,
    http_client: &State<'_, HttpClient>,
    user: User,
    auth_ctx: &AuthCtx,
) -> Result<()> {
    let client = http_client.client();
    let files_to_upload = get_files_to_upload(&database, &auth_ctx)?;
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

        database.update_file_status(&descriptor.uuid, SyncStatus::InProgress)?;

        let file = fs::read(&descriptor.path).unwrap();
        let (encrypted_data, encrypted_data_hash) = encrypt_data(descriptor, key, file.into())
            .with_context(|| format!("Could not encrypt file {:?}", descriptor.path))?;

        let form = reqwest::multipart::Form::new().part(
            "file",
            Part::bytes(encrypted_data)
                .file_name(descriptor.path.to_owned())
                .mime_str("image/jpeg")
                .unwrap(),
        );

        debug!("Sending file: {:?}", &descriptor.path);
        let upload_response = client
            .post(format!(
                "{}/files/{}/data",
                http_client.url(),
                descriptor.uuid
            ))
            .header("Authorization", auth_ctx.get_auth_token())
            .header("sha256_checksum", encrypted_data_hash)
            .multipart(form)
            .send()
            .await;

        match upload_response {
            Ok(response) if response.status().is_success() => {
                database.update_file_status(&descriptor.uuid, SyncStatus::Done)?;
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

fn get_files_to_upload(
    database: &State<Database>,
    auth_ctx: &AuthCtx,
) -> Result<Vec<FileDescriptorWithDecodedKey>> {
    Ok(database
        .find_files_by_sync_status(SyncStatus::New)?
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
