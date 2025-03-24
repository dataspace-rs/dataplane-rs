use bon::Builder;
use derive_more::{From, Into};
use edc_dataplane_core::signaling::DataAddress;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Builder)]
pub struct Edr {
    #[builder(into)]
    pub token_id: TokenId,
    #[builder(into)]
    pub refresh_token_id: RefreshTokenId,
    pub data_address: DataAddress,
}

#[derive(Builder, FromRow, Clone, Debug, PartialEq)]
pub struct EdrEntry {
    #[builder(into)]
    pub transfer_id: String,
    #[builder(into)]
    pub token_id: TokenId,
    #[builder(into)]
    pub refresh_token_id: RefreshTokenId,
}

#[derive(From, Into, Serialize, Clone, Copy, Debug, sqlx::Type, PartialEq)]
#[sqlx(transparent)]
pub struct TokenId(Uuid);

#[derive(From, Into, Clone, Debug, sqlx::Type, PartialEq, Copy)]
#[sqlx(transparent)]
pub struct RefreshTokenId(Uuid);

#[derive(Builder, Serialize, Debug, Deserialize)]
pub struct EdrClaims {
    pub jti: Uuid,
    pub aud: String,
    pub iss: String,
    pub sub: String,
    exp: i64,
    iat: i64,
    pub transfer_id: String,
}

impl EdrClaims {
    pub fn transfer_id(&self) -> &str {
        &self.transfer_id
    }

    pub fn jti(&self) -> Uuid {
        self.jti
    }
}
