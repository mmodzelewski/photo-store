use std::fs;

use anyhow::Result;
use reqwest::multipart::{Form, Part};
use uuid::Uuid;

#[tokio::test]
async fn dev() -> Result<()> {
    let client = reqwest::Client::new();
    let user_id = Uuid::new_v4().to_string();
    println!("user_id: {}", user_id);
    let file_id = Uuid::new_v4().to_string();
    println!("file_id: {}", file_id);

    let response = client
        .post(format!("http://localhost:3000/u/{user_id}/files"))
        .header("Content-Type", "application/json")
        .body(format!(
            r#"{{
            "path": "test-path",
            "uuid": "{file_id}",
            "date": "2021-03-28T00:12:00+02:00",
            "sha256": "f2ca1bb6c7e907d06dafe4687e579fce76b37e4e93b7605022da52e6ccc26fd2"
        }}"#
        ))
        .send()
        .await?;
    println!("Response: {:?}", response);
    println!("Response: {:?}", response.text().await?);

    if let Ok(file) = fs::read("tests/img.jpg") {
        let response = client
            .post(format!(
                "http://localhost:3000/u/{user_id}/files/{file_id}/data"
            ))
            .multipart(
                Form::new()
                    .text("uuid", "8f664c5d-8751-4b8d-bd07-0b115e97f24a")
                    .part(
                        "file",
                        Part::bytes(file)
                            .file_name("img.jpg")
                            .mime_str("image/jpeg")?,
                    ),
            )
            .send()
            .await?;

        println!("Response: {:?}", response);
        println!("Response: {:?}", response.text().await?);
    }

    Ok(())
}
