use dtos::auth::{LoginRequest, LoginResponse};
use log::debug;

use crate::error::Result;

use super::HttpClient;

pub async fn login(http_client: HttpClient, data: &LoginRequest) -> Result<LoginResponse> {
    let client = http_client.client;
    let response = client
        .post(format!("{}/login", http_client.url))
        .header("Content-Type", "application/json")
        .json(&data)
        .send()
        .await
        .unwrap();
    debug!("{:?}", response);

    return response.json().await.map_err(|e| e.into());
}
