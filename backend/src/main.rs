use axum::Router;
use http::header::AUTHORIZATION;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;
use tracing_subscriber::EnvFilter;

use config::Config;
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
    config: Config,
    google_auth: auth::google::GoogleAuth,
    http_client: HttpClient,
}

#[derive(Clone)]
pub struct HttpClient {
    client: reqwest::Client,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config = Config::load()?;
    let pool = database::init_db(&config.database).await?;
    let google_auth = auth::google::GoogleAuth::new();
    let http_client = HttpClient {
        client: reqwest::Client::new(),
    };
    let state = AppState {
        db: pool,
        config,
        google_auth,
        http_client,
    };

    let file_routes = file::routes(state.clone())
        .route_layer(axum::middleware::from_fn(auth::middleware::require_auth))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth::middleware::ctx_resolver,
        ));

    let app = Router::new()
        .merge(file_routes)
        .nest("/auth", auth::routes(state.clone()))
        .merge(user::routes(state.clone()))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_headers([AUTHORIZATION]),
        );

    info!("Listening on localhost:3000");
    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();

    Ok(())
}
