use axum::{
    extract::{DefaultBodyLimit, Json, Multipart, Path, State},
    routing::{get, post},
    Router,
};
use file::NewFile;
use tower_http::limit::RequestBodyLimitLayer;
use tracing::{debug, info};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

use database::DbPool;
use endpoints::{get_data, list_uploads, upload};
use error::Result;

mod config;
mod database;
mod endpoints;
mod error;
mod file;

#[derive(Clone)]
struct AppState {
    db: DbPool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let pool = database::init_db().await?;

    let app = Router::new()
        .route("/", get(get_data))
        .route("/upload", post(upload).get(list_uploads))
        .route("/u/:id/files", post(file_meta_upload))
        .route("/u/:user_id/files/:file_id/data", post(upload_file))
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
    let exists = file::repository::FileRepository::exists(&db, &file.uuid).await?;

    if exists {
        println!("File already exists");
        return Ok(());
    }

    println!("Saving file");
    file::repository::FileRepository::save(&db, &file).await?;

    Ok(())
}

async fn upload_file(
    State(state): State<AppState>,
    Path((user_id, file_id)): Path<(Uuid, Uuid)>,
    mut multipart: Multipart,
) -> Result<()> {
    debug!("user_id: {:?}", user_id);
    debug!("file_id: {:?}", file_id);
    while let Some(field) = multipart.next_field().await? {
        debug!("got field: {:?}", field.name());
        if Some("file") == field.name() {
            debug!("file content type: {:?}", field.content_type());
            debug!("file file name: {:?}", field.file_name());
            debug!("file headers: {:?}", field.headers());
            let _ = field.bytes().await?;
        }
    }
    Ok(())
}
