use thiserror::Error;
use uuid::Uuid;

use crate::core::{
    db::transfer::TransferStoreRef,
    model::{
        edr::{EdrClaims, RefreshTokenId, TokenId},
        token::{TokenRequest, TokenResponse},
        transfer::TransferStatus,
    },
};

use super::{
    edr::{EdrError, EdrManager},
    token::{TokenError, TokenManager},
};

#[derive(Clone)]
pub struct RefreshManager<T: TokenManager> {
    edrs: EdrManager<T>,
    store: TransferStoreRef,
}

impl<T: TokenManager> RefreshManager<T> {
    pub fn new(edrs: EdrManager<T>, store: TransferStoreRef) -> Self {
        Self { edrs, store }
    }

    pub async fn refresh_token(&self, req: TokenRequest) -> Result<TokenResponse, RefreshError> {
        let claims = self.edrs.tokens.validate::<EdrClaims>(&req.refresh_token)?;

        let mut transfer = self
            .store
            .fetch_by_id(&claims.claims.transfer_id)
            .await?
            .filter(|t| {
                t.status == TransferStatus::Started
                    && t.refresh_token_id == claims.claims.jti.into()
            })
            .ok_or_else(|| {
                RefreshError::Generic(anyhow::anyhow!("Transfer not found or not valid"))
            })?;

        let token_id: TokenId = Uuid::new_v4().into();
        let refresh_token_id: RefreshTokenId = Uuid::new_v4().into();

        let token_response = self
            .edrs
            .issue_token(
                token_id,
                refresh_token_id,
                &claims.claims.sub,
                &claims.claims.transfer_id,
            )
            .map(Ok)?;

        transfer.refresh_token_id = refresh_token_id;
        transfer.token_id = token_id;
        transfer.updated_at = chrono::Utc::now();

        self.store.save(transfer).await?;

        token_response
    }
}

#[derive(Debug, Error)]
pub enum RefreshError {
    #[error(transparent)]
    Token(#[from] TokenError),
    #[error(transparent)]
    Edr(#[from] EdrError),
    #[error(transparent)]
    Generic(#[from] anyhow::Error),
}
