use axum::Router;
use tracing::info;
use tracing_subscriber::EnvFilter;

use database::DbPool;
use error::Result;

mod config;
mod database;
mod endpoints;
mod error;
mod file;
mod middleware;
mod ctx;

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

    let file_routes = file::routes::routes(AppState { db: pool })
        .route_layer(axum::middleware::from_fn(middleware::require_auth));

    let app = Router::new().merge(file_routes);

    info!("Listening on localhost:3000");
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}
