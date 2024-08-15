use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct SaveRsaKeysRequest {
    pub private_key: String,
    pub public_key: String,
}

#[derive(Serialize, Deserialize)]
pub struct PrivateKeyResponse {
    pub value: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct LoginResponse {
    pub user_id: Uuid,
    pub auth_token: String,
}
