use aws_config::BehaviorVersion;
use axum::{Router, http::Request};
use http::header::AUTHORIZATION;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::EnvFilter;

use config::Config;
use database::DbPool;
use error::Result;

mod auth;
mod config;
mod database;
mod entity;
mod error;
mod file;
mod migration;
mod session;
mod ulid;
mod upload;

#[derive(Clone)]
pub struct AppState {
    db: DbPool,
    config: Config,
    s3_client: aws_sdk_s3::Client,
}

const REQUEST_ID_HEADER: &str = "x-request-id";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config = Config::load()?;
    let pool = database::init_db(&config.database).await?;

    let aws_config = aws_config::defaults(BehaviorVersion::latest())
        .region("auto")
        .endpoint_url(&config.storage.url)
        .load()
        .await;
    let s3_client = aws_sdk_s3::Client::new(&aws_config);

    let state = AppState {
        db: pool,
        config,
        s3_client,
    };

    let x_request_id = http::HeaderName::from_static(REQUEST_ID_HEADER);

    tokio::spawn(upload::cleanup_expired_uploads(state.clone()));

    let app = Router::new()
        .merge(file::routes(state.clone()))
        .merge(upload::routes(state.clone()))
        .layer(axum::middleware::from_fn(auth::middleware::require_auth))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth::middleware::session_resolver,
        ))
        .nest("/auth", auth::routes(state.clone()))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_headers([AUTHORIZATION]),
        )
        .layer(PropagateRequestIdLayer::new(x_request_id.clone()))
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &Request<_>| {
                let request_id = request
                    .headers()
                    .get(REQUEST_ID_HEADER)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("unknown");

                tracing::info_span!(
                    "request",
                    method = %request.method(),
                    path = %request.uri().path(),
                    request_id = %request_id,
                )
            }),
        )
        .layer(SetRequestIdLayer::new(x_request_id, MakeRequestUuid));

    info!("Listening on localhost:3000");
    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();

    Ok(())
}
