use anyhow::{anyhow, Context};
use crypto::{decode_encryption_key, decrypt_data, encrypt_data};
use dtos::file::{FileMetadata, FilesUploadRequest};
use log::debug;
use reqwest::multipart::Part;
use std::collections::HashSet;
use std::fs;

use crate::error::Result;
use crate::files::FileDescriptorWithDecodedKey;
use crate::state::SyncedAppState;
use crate::{database::Database, http::HttpClient};

#[tauri::command]
pub(crate) async fn sync_images(
    database: tauri::State<'_, Database>,
    app_state: tauri::State<'_, SyncedAppState>,
    http_client: tauri::State<'_, HttpClient>,
) -> Result<()> {
    debug!("sync_images called");

    let state = app_state.read();
    let user = state.user.context("User is not logged in")?;
    let auth_ctx = state.auth_ctx.context("User is not authenticated")?;

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

    let descriptors_with_keys: Vec<_> = database
        .get_indexed_images()?
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
        .collect();

    let local_file_uuids: HashSet<_> = descriptors_with_keys
        .iter()
        .map(|desc| desc.descriptor().uuid)
        .collect();

    let dirs = database.get_directories()?;
    // todo: take proper directory for downloads
    let output_dir = dirs.first().ok_or(anyhow!("No directories selected"))?;
    for remote_file in remote_files.iter() {
        if !local_file_uuids.contains(&remote_file.uuid) {
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

    let image_metadata = descriptors_with_keys
        .iter()
        .map(|desc| desc.descriptor().into())
        .collect();

    let body = FilesUploadRequest {
        user_id: user.id,
        files: image_metadata,
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
    for descriptor_with_key in descriptors_with_keys {
        let descriptor = descriptor_with_key.descriptor();
        let key = descriptor_with_key.key();
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
        let res = client
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
        debug!("{:?}", res);
    }
    Ok(())
}
