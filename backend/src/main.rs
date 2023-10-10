use axum::{
    extract::{DefaultBodyLimit, Json, Path, State},
    routing::{get, post},
    Router,
};
use sqlx::postgres::PgPoolOptions;
use time::OffsetDateTime;
use tower_http::limit::RequestBodyLimitLayer;

use endpoints::{get_data, list_uploads, upload};
use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::error::Result;

mod config;
mod endpoints;
mod error;

#[derive(Clone)]
struct AppState {
    db: sqlx::Pool<sqlx::Postgres>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://postgres:postgres@localhost:5432/photo_store_test")
        .await?;

    sqlx::migrate!("db/migrations").run(&pool).await.unwrap();

    let app = Router::new()
        .route("/", get(get_data))
        .route("/upload", post(upload).get(list_uploads))
        .route("/u/:id/files", post(file_meta_upload))
        .layer(DefaultBodyLimit::disable())
        .layer(RequestBodyLimitLayer::new(250 * 1024 * 1024))
        .with_state(AppState { db: pool });

    info!("Listening on localhost:3000");
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}

#[derive(serde::Deserialize)]
struct NewFile {
    path: String,
    uuid: uuid::Uuid,
    #[serde(with = "time::serde::iso8601")]
    date: OffsetDateTime,
    sha256: String,
}

#[derive(sqlx::Type)]
#[sqlx(type_name = "file_state")]
enum FileState {
    New,
    SyncInProgress,
    Synced,
}

async fn file_meta_upload(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    Json(file): Json<NewFile>,
) -> Result<()> {
    println!("{}", user_id);
    println!("{}", file.path);
    println!("{}", file.uuid);
    println!("{}", file.date);

    let db = state.db;
    let count = sqlx::query!("SELECT count(1) FROM file WHERE uuid = $1", &file.uuid)
        .fetch_one(&db)
        .await?;
    let count = count.count.unwrap_or(0);
    println!("{}", count);

    if count != 0 {
        println!("File already exists");
        return Ok(());
    }

    println!("Saving file");
    let query = sqlx::query!(
        "INSERT INTO file (path, name, state, uuid, created_at, sha256) VALUES ($1, $2, $3, $4, $5, $6)",
        &file.path,
        &file.path,
        FileState::New as _,
        &file.uuid,
        &file.date,
        &file.sha256
    );

    let result = query.execute(&db).await;
    if let Err(e) = result {
        println!("Error: {}", e);
        return Ok(());
    }

    Ok(())
}
