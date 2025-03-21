use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct TokenRequest {
    pub refresh_token: String,
    pub client_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: String,
}
