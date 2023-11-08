use std::fs;

use anyhow::Result;
use reqwest::multipart::{Form, Part};
use uuid::Uuid;

#[tokio::test]
async fn dev_upload() -> Result<()> {
    let client = reqwest::Client::new();
    let user_id = Uuid::new_v4().to_string();
    println!("user_id: {}", user_id);
    let file_id = Uuid::new_v4().to_string();
    println!("file_id: {}", file_id);

    let response = client
        .post(format!("http://localhost:3000/files/metadata"))
        .header("Content-Type", "application/json")
        .header("Authorization", "29c48a07-e255-44c4-ada9-40be7532c6bb")
        .body(format!(
            r#"{{
                "user_id": "{user_id}",
                "items": [
                    {{
                        "path": "test-path",
                        "uuid": "{file_id}",
                        "date": "2021-03-28T00:12:00+02:00",
                        "sha256": "f2ca1bb6c7e907d06dafe4687e579fce76b37e4e93b7605022da52e6ccc26fd2"
                    }}
                ]
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
            .header("Authorization", "test-token")
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

#[derive(serde::Deserialize, Debug)]
struct UserResponse {
    user_id: String,
}

#[derive(serde::Deserialize, Debug)]
struct LoginResponse {
    auth_token: String,
}

#[tokio::test]
async fn dev_user() -> Result<()> {
    let client = reqwest::Client::new();
    let uuid = Uuid::new_v4().to_string();
    let username = "test-user".to_string() + uuid.as_str();
    println!("username: {}", username);

    let response = client
        .post("http://localhost:3000/user")
        .header("Content-Type", "application/json")
        .body(format!(
            r#"{{"username": "{username}", "password": "test"}}"#
        ))
        .send()
        .await?;

    println!("Response: {:?}", response);
    println!("Response status: {:?}", response.status());
    let user_response = response.json::<UserResponse>().await?;
    println!("Response body: {:?}", user_response);

    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:3000/login")
        .header("Content-Type", "application/json")
        .body(format!(
            r#"{{"username": "{username}", "password": "test"}}"#
        ))
        .send()
        .await?;

    println!("Response: {:?}", response);
    println!("Response status: {:?}", response.status());
    let login_response = response.json::<LoginResponse>().await?;
    println!("Response body: {:?}", login_response);

    Ok(())
}
