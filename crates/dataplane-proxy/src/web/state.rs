use edc_dataplane_core::core::service::transfer::TransferService;

use crate::service::{edr::EdrManager, refresh::RefreshManager, token::TokenManager};

#[derive(Clone)]
pub struct Context<T: TokenManager + Clone> {
    transfers: TransferService,
    tokens: T,
    refresh_manager: RefreshManager<T>,
}

impl<T: TokenManager + Clone> Context<T> {
    pub fn new(transfers: TransferService, tokens: T, refresh_manager: RefreshManager<T>) -> Self {
        Self {
            transfers,
            tokens,
            refresh_manager,
        }
    }

    pub fn transfers(&self) -> &TransferService {
        &self.transfers
    }

    pub fn tokens(&self) -> &T {
        &self.tokens
    }

    pub fn refresh_manager(&self) -> &RefreshManager<T> {
        &self.refresh_manager
    }

    pub fn edrs(&self) -> &EdrManager<T> {
        &self.refresh_manager.edrs
    }
}
