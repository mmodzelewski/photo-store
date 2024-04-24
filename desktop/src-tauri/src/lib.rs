mod database;
mod error;
mod handlers;
mod http;
mod image;

use crate::image::image_protocol::image_protocol_handler;
use database::Database;
use error::{Error, Result};
use handlers::AppState;
use http::HttpClient;
use log::debug;
use std::{fs, sync::Mutex};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            handlers::sync_images,
            handlers::save_images_dirs,
            handlers::has_images_dirs,
            handlers::get_images,
            handlers::login,
        ])
        .register_uri_scheme_protocol("image", image_protocol_handler)
        .setup(|app| {
            env_logger::Builder::new()
                .filter_level(log::LevelFilter::Trace)
                .init();

            let path = app.path().app_data_dir().ok().ok_or(Error::Runtime(
                "Could not get app data directory".to_owned(),
            ))?;
            fs::create_dir_all(&path)?;

            app.manage(Database::init(path)?);
            update_scopes(app)?;
            app.manage(AppState {
                user_data: Default::default(),
                http_client: Mutex::new(HttpClient::new("http://localhost:3000")),
            });
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
