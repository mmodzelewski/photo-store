use crypto::encrypt_data;
use dtos::file::FilesUploadRequest;
use log::debug;
use reqwest::multipart::Part;
use std::fs;

use crate::error::{Error, Result};
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
    let user = state
        .user
        .ok_or(Error::Generic("User is not logged in".to_owned()))?;
    let auth_ctx = state
        .auth_ctx
        .ok_or(Error::Generic("User is not authenticated".to_owned()))?;

    let descriptors_with_keys: Vec<_> = database
        .get_indexed_images()?
        .into_iter()
        .map(|desc| {
            let key = crypto::decode_encryption_key(&desc.key, auth_ctx.decrypt(), &desc).unwrap();
            FileDescriptorWithDecodedKey::new(desc, key)
        })
        .collect();

    let image_metadata = descriptors_with_keys
        .iter()
        .map(|desc| desc.descriptor().into())
        .collect();

    let body = FilesUploadRequest {
        user_id: user.id,
        files: image_metadata,
    };
    debug!("Sending metadata: {:?}", body);

    let client = http_client.client();

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
        let (encrypted_data, encrypted_data_hash) = encrypt_data(descriptor, key, file.into())?;

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
    return Ok(());
}
