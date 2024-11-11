use std::fs;
use std::path::Path;
use std::sync::RwLock;

use base64ct::{Base64, Encoding};
use dtos::auth::{PrivateKeyResponse, SaveRsaKeysRequest};
use log::debug;
use reqwest::multipart::Part;
use rsa::RsaPublicKey;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_shell::ShellExt;
use time::format_description::FormatItem;
use time::macros::format_description;
use time::OffsetDateTime;
use tiny_http::{Header, Server};
use url::Url;
use uuid::Uuid;
use walkdir::{DirEntry, WalkDir};

use crypto::encrypt_data;
use dtos::file::FilesUploadRequest;

use crate::auth::{AuthCtx, AuthStore};
use crate::database::Database;
use crate::error::{Error, Result};
use crate::files::{FileDescriptor, FileDescriptorWithDecodedKey};
use crate::http::HttpClient;

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

#[derive(Clone, Debug)]
pub(crate) struct User {
    pub id: Uuid,
    pub name: String,
}

#[tauri::command]
pub(crate) fn get_status(
    database: tauri::State<Database>,
    app_state: tauri::State<SyncedAppState>,
) -> Result<String> {
    debug!("Getting app status");
    let state = app_state.read();
    if state.auth_ctx.is_none() {
        return Ok("before_login".to_owned());
    }
    if database.has_images_dirs()? {
        Ok("directories_selected".to_owned())
    } else {
        Ok("after_login".to_owned())
    }
}

#[tauri::command]
pub(crate) async fn save_images_dirs(
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
        let (encrypted_data, encrypted_data_hash) =
            encrypt_data(descriptor, key, file.into())?;

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

#[tauri::command]
pub(crate) fn has_images_dirs(database: tauri::State<Database>) -> Result<bool> {
    debug!("Checking images dirs");
    database.has_images_dirs()
}

#[tauri::command]
pub(crate) async fn authenticate(
    app_handle: AppHandle,
    database: tauri::State<'_, Database>,
    app_state: tauri::State<'_, SyncedAppState>,
) -> Result<()> {
    let server = Server::http("127.0.0.1:0").unwrap();
    let ip = server.server_addr().to_ip().unwrap();

    debug!("Listening on 127.0.0.1:{}", ip.port());
    let redirect_uri = format!("http://127.0.0.1:{}", ip.port());

    app_handle
        .shell()
        .open(
            format!(
                "http://localhost:5173/auth/desktop?redirect_uri={}",
                redirect_uri
            ),
            None,
        )
        .unwrap();

    if let Ok(request) = server.recv() {
        let url = Url::parse(&format!("http://localhost{}", request.url())).unwrap();

        let (_, auth_token) = url
            .query_pairs()
            .find(|(key, _)| key == "auth_token")
            .unwrap();
        let (_, user_id) = url.query_pairs().find(|(key, _)| key == "user_id").unwrap();
        let user_id = Uuid::parse_str(&user_id).unwrap();

        let auth_store = AuthStore::new(auth_token.to_string());
        auth_store.save(&user_id)?;

        let user = User {
            id: user_id,
            name: "".to_owned(),
        };
        database.save_user(&user)?;
        app_state.replace_user(user);

        let done_url = "http://localhost:5173/auth/desktop/complete";
        let response = tiny_http::Response::empty(303)
            .with_header(Header::from_bytes(&b"Location"[..], &done_url.as_bytes()[..]).unwrap());
        request.respond(response).unwrap();

        debug!("Listener closed.");
    }
    app_handle.emit("authenticated", ())?;

    Ok(())
}

#[tauri::command]
pub(crate) async fn get_private_key(
    passphrase: String,
    app_state: tauri::State<'_, SyncedAppState>,
    http_client: tauri::State<'_, HttpClient>,
) -> Result<()> {
    debug!("Initiating private key");
    let client = http_client.client();

    let state = app_state.read();
    let user = state
        .user
        .ok_or(Error::Generic("User is not logged in".to_owned()))?;
    let auth_store = AuthStore::load(&user.id)?;
    let auth_token = auth_store.get_auth_token();

    let private_key = client
        .get(format!("{}/auth/keys", http_client.url()))
        .header("Authorization", auth_token)
        .send()
        .await?
        .json::<PrivateKeyResponse>()
        .await?;

    let (cipher, nonce) = crypto::generate_cipher(&user.id, &passphrase)?;
    let private_key = if let Some(private_key_encrypted) = private_key.value {
        debug!("decrypting existing key");
        let pk_der = crypto::decrypt_data_raw(&private_key_encrypted, &cipher, &nonce)?;
        crypto::rsa::from_der(&pk_der)?
    } else {
        debug!("creating new key");
        let private_key = crypto::rsa::generate_key();

        let pk_bytes = crypto::rsa::to_der(&private_key)?;
        let private_key_encrypted = crypto::encrypt_data_raw(&pk_bytes, &cipher, &nonce);
        debug!("new key created");

        let body = SaveRsaKeysRequest {
            private_key: private_key_encrypted.clone(),
            public_key: crypto::rsa::to_public_key_pem(&private_key)?,
        };
        client
            .post(format!("{}/auth/keys", http_client.url()))
            .header("Content-Type", "application/json")
            .header("Authorization", auth_token)
            .body(serde_json::to_string(&body).unwrap())
            .send()
            .await?;
        debug!("key sent");
        private_key
    };

    let auth_store = auth_store.with_private_key(private_key);
    auth_store.save(&user.id)?;
    let ctx: AuthCtx = auth_store.try_into()?;
    app_state.replace_auth_ctx(ctx);

    Ok(())
}

#[derive(Clone)]
pub(crate) struct AppState {
    pub user: Option<User>,
    pub auth_ctx: Option<AuthCtx>,
}

pub(crate) struct SyncedAppState(RwLock<AppState>);

impl SyncedAppState {
    pub(crate) fn new(user: Option<User>, auth_ctx: Option<AuthCtx>) -> Self {
        Self(RwLock::new(AppState { user, auth_ctx }))
    }

    fn read(&self) -> AppState {
        self.0.read().unwrap().clone()
    }

    fn replace_auth_ctx(&self, ctx: AuthCtx) {
        self.0.write().unwrap().auth_ctx.replace(ctx);
    }

    fn replace_user(&self, user: User) {
        self.0.write().unwrap().user.replace(user);
    }
}
