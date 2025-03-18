use axum::extract::FromRef;

use crate::core::service::{
    refresh::RefreshManager, token::TokenManager, transfer::TransferManager,
};

#[derive(Clone)]
pub struct Context<T: TokenManager + Clone> {
    transfer_manager: TransferManager<T>,
    tokens: T,
    refresh_manager: RefreshManager<T>,
}

impl<T: TokenManager + Clone> Context<T> {
    pub fn new(
        transfer_manager: TransferManager<T>,
        tokens: T,
        refresh_manager: RefreshManager<T>,
    ) -> Self {
        Self {
            transfer_manager,
            tokens,
            refresh_manager,
        }
    }

    pub fn tokens(&self) -> &T {
        &self.tokens
    }

    pub fn transfer_manager(&self) -> &TransferManager<T> {
        &self.transfer_manager
    }

    pub fn refresh_manager(&self) -> &RefreshManager<T> {
        &self.refresh_manager
    }
}

impl<T: TokenManager + Clone> FromRef<Context<T>> for TransferManager<T> {
    fn from_ref(ctx: &Context<T>) -> TransferManager<T> {
        ctx.transfer_manager.clone()
    }
}
