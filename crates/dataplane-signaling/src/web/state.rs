use axum::extract::FromRef;
use edc_dataplane_core::core::service::transfer::TransferService;

#[derive(Clone)]
pub struct Context {
    transfer_manager: TransferService,
}

impl Context {
    pub fn new(transfer_manager: TransferService) -> Self {
        Self { transfer_manager }
    }

    pub fn transfer_manager(&self) -> &TransferService {
        &self.transfer_manager
    }
}

impl FromRef<Context> for TransferService {
    fn from_ref(ctx: &Context) -> TransferService {
        ctx.transfer_manager.clone()
    }
}
