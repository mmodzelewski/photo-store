use anyhow::{anyhow, Context};
use log::debug;
use std::sync::RwLock;
use uuid::Uuid;

use crate::auth::AuthCtx;
use crate::database::Database;
use crate::error::{Error, Result};

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

#[derive(Clone, Debug)]
pub(crate) struct User {
    pub id: Uuid,
    pub name: String,
}
#[derive(Clone)]
pub(crate) struct AppState {
    pub user: Option<User>,
    pub auth_ctx: Option<AuthCtx>,
}

impl AppState {
    pub fn get_authenticated_user(self) -> Result<(User, AuthCtx)> {
        let user = self.user.context("User is not logged in")?;
        let auth_ctx = self.auth_ctx.context("User is not authenticated")?;
        Ok((user, auth_ctx))
    }
}

pub struct SyncedAppState(RwLock<AppState>);

impl SyncedAppState {
    pub(crate) fn new(user: Option<User>, auth_ctx: Option<AuthCtx>) -> Self {
        Self(RwLock::new(AppState { user, auth_ctx }))
    }

    pub fn read(&self) -> AppState {
        self.0.read().unwrap().clone()
    }

    pub fn replace_auth_ctx(&self, ctx: AuthCtx) {
        self.0.write().unwrap().auth_ctx.replace(ctx);
    }

    pub fn replace_user(&self, user: User) {
        self.0.write().unwrap().user.replace(user);
    }
}
