use edc_dataplane_core::core::{
    db::transfer::TransferRepoRef,
    model::transfer::{Transfer, TransferStatus},
};
use thiserror::Error;
use uuid::Uuid;

use crate::model::{
    edr::{EdrClaims, EdrEntry, RefreshTokenId, TokenId},
    token::{TokenRequest, TokenResponse},
};

use super::{
    edr::{EdrError, EdrManager},
    token::{TokenError, TokenManager},
};

#[derive(Clone)]
pub struct RefreshManager<T: TokenManager> {
    pub(crate) edrs: EdrManager<T>,
    store: TransferRepoRef,
}

impl<T: TokenManager> RefreshManager<T> {
    pub fn new(edrs: EdrManager<T>, store: TransferRepoRef) -> Self {
        Self { edrs, store }
    }

    async fn get_transfer(&self, id: &str) -> Result<Transfer, RefreshError> {
        self.store
            .fetch_by_id(id)
            .await?
            .filter(|t| t.status == TransferStatus::Started)
            .ok_or_else(|| {
                RefreshError::Generic(anyhow::anyhow!("Transfer not found or not valid"))
            })
    }

    async fn get_edr_entry(&self, claims: &EdrClaims) -> Result<EdrEntry, RefreshError> {
        self.edrs
            .get_by_transfer_id(&claims.transfer_id)
            .await?
            .filter(|t| t.refresh_token_id == claims.jti.into())
            .ok_or_else(|| {
                RefreshError::Generic(anyhow::anyhow!("Transfer not found or not valid"))
            })
    }

    pub async fn refresh_token(&self, req: TokenRequest) -> Result<TokenResponse, RefreshError> {
        let claims = self.edrs.tokens.validate::<EdrClaims>(&req.refresh_token)?;

        let _transfer = self.get_transfer(&claims.claims.transfer_id).await?;

        let mut edr_entry = self.get_edr_entry(&claims.claims).await?;

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

        edr_entry.refresh_token_id = refresh_token_id;
        edr_entry.token_id = token_id;

        self.edrs.save(edr_entry).await?;

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
