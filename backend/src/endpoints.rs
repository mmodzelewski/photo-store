use std::path::Path;

use aws_sdk_s3::{
    primitives::ByteStream,
    types::{CompletedMultipartUpload, CompletedPart},
};
use axum::extract::Multipart;

use crate::config::Config;

#[allow(dead_code)]
pub(crate) async fn list_uploads() -> &'static str {
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

    let result = client
        .list_multipart_uploads()
        .bucket(&local_config.bucket_name)
        .send()
        .await
        .unwrap();

    for upload in result.uploads().unwrap() {
        println!("{:?}", upload);
        client
            .abort_multipart_upload()
            .bucket(&local_config.bucket_name)
            .key(upload.key().unwrap())
            .upload_id(upload.upload_id().unwrap())
            .send()
            .await
            .unwrap();
    }

    return "hello data";
}

#[allow(dead_code)]
pub(crate) async fn upload(mut multipart: Multipart) {
    let local_config = Config::load().await;
    if local_config.is_err() {
        println!("{:?}", local_config);
        return;
    }
    let local_config = local_config.unwrap();
    let key = "test-image2";

    let config = aws_config::from_env()
        .region("auto")
        .endpoint_url(local_config.r2_url)
        .load()
        .await;
    let client = aws_sdk_s3::Client::new(&config);
    let output = client
        .create_multipart_upload()
        .bucket(&local_config.bucket_name)
        .key(key)
        .content_type("image/jpeg")
        .send()
        .await
        .unwrap();
    let upload_id = output.upload_id().unwrap();

    let mut part_number = 1;
    let mut upload_parts: Vec<CompletedPart> = Vec::new();

    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();
        let file_name = field.file_name().unwrap().to_string();
        let content_type = field.content_type().unwrap().to_string();
        let data = field.bytes().await.unwrap();

        println!(
            "Length of `{}` (`{}`: `{}`) is {} bytes",
            name,
            file_name,
            content_type,
            data.len()
        );

        let stream = ByteStream::from(data);

        let completed = client
            .upload_part()
            .key(key)
            .bucket(&local_config.bucket_name)
            .upload_id(upload_id)
            .body(stream)
            .part_number(part_number)
            .send()
            .await
            .unwrap();

        upload_parts.push(
            CompletedPart::builder()
                .e_tag(completed.e_tag().unwrap_or_default())
                .part_number(part_number)
                .build(),
        );
        part_number += 1;
    }
    let completed = CompletedMultipartUpload::builder()
        .set_parts(Some(upload_parts))
        .build();

    let complete = client
        .complete_multipart_upload()
        .bucket(&local_config.bucket_name)
        .key(key)
        .multipart_upload(completed)
        .upload_id(upload_id)
        .send()
        .await
        .unwrap();
    dbg!(complete);
}

#[allow(dead_code)]
pub(crate) async fn get_data() -> &'static str {
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
