use axum::{
    extract::{DefaultBodyLimit, Json, Path, State},
    Router,
    routing::{get, post},
};
use sqlx::postgres::PgPoolOptions;
use time::OffsetDateTime;
use tower_http::limit::RequestBodyLimitLayer;

use endpoints::{get_data, list_uploads, upload};

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
        "INSERT INTO file (path, name, uuid, created_at, sha256) VALUES ($1, $2, $3, $4, $5)",
        &file.path,
        &file.path,
        &file.uuid,
        &file.date,
        &file.sha256
    );

    query.execute(&db).await?;

    Ok(())
}
