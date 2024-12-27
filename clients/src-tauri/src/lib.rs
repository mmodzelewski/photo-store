mod auth;
mod database;
mod error;
mod files;
mod handlers;
mod http;
mod image;
mod state;
mod sync;

use crate::{auth::AuthCtx, image::image_protocol::image_protocol_handler};
use anyhow::Context;
use database::Database;
use error::{Error, Result};
use http::HttpClient;
use log::debug;
use state::SyncedAppState;
use std::fs;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            sync::sync_images,
            handlers::save_images_dirs,
            handlers::has_images_dirs,
            handlers::get_images,
            auth::handlers::authenticate,
            auth::handlers::get_private_key,
            state::get_status,
        ])
        .register_uri_scheme_protocol("image", |ctx, request| {
            let app = ctx.app_handle();
            image_protocol_handler(app, request)
        })
        .setup(|app| {
            env_logger::Builder::new()
                .filter_level(log::LevelFilter::Info)
                .parse_default_env()
                .init();

            let path = app
                .path()
                .app_data_dir()
                .context("Could not get app data directory")?;
            fs::create_dir_all(&path)?;

            let database = Database::init(path)?;
            let user = database.get_user()?;
            debug!("Logged in user: {:?}", user);

            app.manage(database);
            update_scopes(app)?;

            let auth_ctx = user
                .as_ref()
                .map(|user| auth::AuthStore::load(&user.id))
                .transpose()?
                .and_then(|store| AuthCtx::try_from(store).ok());

            app.manage(SyncedAppState::new(user, auth_ctx));

            let api_url = option_env!("PHOTO_STORE_API_URL").unwrap_or("http://localhost:3000");
            debug!("API URL: {}", api_url);
            app.manage(HttpClient::new(&api_url));

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn update_scopes(app: &tauri::App) -> Result<()> {
    let database = app.state::<Database>();
    let dirs = database.get_directories()?;

    for dir in dirs {
        debug!("Updating scope for {:?}", dir);
        app.asset_protocol_scope()
            .allow_directory(dir, true)
            .context("Could not allow dir for protocol scope")?;
    }

    Ok(())
}
