mod config;
mod endpoints;

use axum::{
    extract::{DefaultBodyLimit, Json, Path, State},
    routing::{get, post},
    Router,
};
use endpoints::{get_data, list_uploads, upload};
use sqlx::postgres::PgPoolOptions;
use tower_http::limit::RequestBodyLimitLayer;

#[derive(Clone)]
struct AppState {
    db: sqlx::Pool<sqlx::Postgres>,
}

#[tokio::main]
async fn main() {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://postgres:postgres@localhost:5432/photo_store_test")
        .await
        .unwrap();

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
}

#[derive(serde::Deserialize)]
struct NewFile {
    name: String,
    uuid: uuid::Uuid,
}

async fn file_meta_upload(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(file): Json<NewFile>,
) {
    println!("{}", id);
    println!("{}", file.name);
    println!("{}", file.uuid);
    let db = state.db;
    let count = sqlx::query!("SELECT count(1) FROM file WHERE uuid = $1", &file.uuid)
        .fetch_one(&db)
        .await
        .unwrap();
    let count = count.count.unwrap_or(0);
    println!("{}", count);
}
