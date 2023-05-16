use std::path::Path;

use aws_sdk_s3::primitives::ByteStream;
use axum::{routing::get, Router};

const URL: &str = "";

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", get(get_data));
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn get_data<'a>() -> &'a str {
    let config = aws_config::from_env()
        .region("auto")
        .endpoint_url(URL)
        .load()
        .await;
    let client = aws_sdk_s3::Client::new(&config);
    let result = client.list_buckets().send().await;
    println!("{:?}", result);

    let body = ByteStream::from_path(Path::new("")).await;
    let result = client
        .put_object()
        .bucket("photo-store-test")
        .key("test-image")
        .content_type("image/jpeg")
        .body(body.unwrap())
        .send()
        .await;

    println!("{:?}", result);
    return "hello data";
}
