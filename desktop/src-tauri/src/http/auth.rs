use dtos::auth::{LoginRequest, LoginResponse};
use log::debug;

use crate::error::Result;

pub async fn login(data: &LoginRequest) -> Result<LoginResponse> {
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:3000/login")
        .header("Content-Type", "application/json")
        .json(&data)
        .send()
        .await
        .unwrap();
    debug!("{:?}", response);

    return response.json().await.map_err(|e| e.into());
}
