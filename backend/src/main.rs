mod config;
mod auth;

use std::path::Path;

use aws_sdk_s3::primitives::ByteStream;
use axum::{routing::get, Router};

use crate::config::Config;

#[tokio::main]
async fn main() {
    let _ = auth::github::setup_github().await;
    let app = Router::new()
        .route("/", get(get_data))
        .route("/redirect", get(auth::github::redirect))
        .route("/user", get(auth::github::get_user_data))
        ;
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn get_data() -> &'static str {
    let local_config = Config::load().await;
    if local_config.is_err() {
        println!("{:?}", local_config);
        return "error";
    }
    let local_config = local_config.unwrap();

    let config = aws_config::from_env()
        .region("auto")
        .endpoint_url(local_config.r2_url)
        .load()
        .await;
    let client = aws_sdk_s3::Client::new(&config);
    let result = client.list_buckets().send().await;
    println!("{:?}", result);

    let body = ByteStream::from_path(Path::new("")).await;
    let result = client
        .put_object()
        .bucket(local_config.bucket_name)
        .key("test-image")
        .content_type("image/jpeg")
        .body(body.unwrap())
        .send()
        .await;

    println!("{:?}", result);
    return "hello data";
}
