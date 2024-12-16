use bon::Builder;
use sqlx::{prelude::FromRow, types::Json};

use crate::signaling::DataAddress;

use super::edr::{RefreshTokenId, TokenId};

#[derive(Builder, Clone, Debug, FromRow, PartialEq)]
pub struct Transfer {
    pub id: String,
    pub status: TransferStatus,
    #[builder(into)]
    pub source: Json<DataAddress>,
    pub token_id: TokenId,
    pub refresh_token_id: RefreshTokenId,
    #[builder(default)]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[builder(default)]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, sqlx::Type, PartialEq)]
pub enum TransferStatus {
    Started,
    Suspended,
}
