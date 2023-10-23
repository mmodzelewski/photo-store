use axum::Router;
use tracing::info;
use tracing_subscriber::EnvFilter;

use database::DbPool;
use error::Result;

mod auth;
mod config;
mod ctx;
mod database;
mod endpoints;
mod error;
mod file;
mod user;

#[derive(Clone)]
pub struct AppState {
    db: DbPool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let pool = database::init_db().await?;
    let state = AppState { db: pool };

    let file_routes = file::routes(state.clone())
        .route_layer(axum::middleware::from_fn(auth::middleware::require_auth))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth::middleware::ctx_resolver,
        ));

    let app = Router::new()
        .merge(file_routes)
        .merge(auth::routes(state.clone()))
        .merge(user::routes(state.clone()));

    info!("Listening on localhost:3000");
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}
