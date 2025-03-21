use async_trait::async_trait;
use bon::{builder, Builder};
use miwa::derive::interface;

use crate::core::model::transfer::{Transfer, TransferStatus};

#[cfg(test)]
use mockall::{automock, predicate::*};

#[async_trait]
#[interface]
#[cfg_attr(test, automock)]
pub trait TransferRepo {
    async fn save(&self, transfer: Transfer) -> anyhow::Result<()>;
    async fn fetch_by_id(&self, transfer_id: &str) -> anyhow::Result<Option<Transfer>>;
    async fn delete(&self, transfer_id: &str) -> anyhow::Result<()>;
    async fn query(&self, query: TransferQuery) -> anyhow::Result<Vec<Transfer>>;
    async fn change_status(
        &self,
        transfer_id: String,
        status: TransferStatus,
    ) -> anyhow::Result<()>;
}

#[derive(Builder)]
pub struct TransferQuery {
    #[builder(default = 50)]
    pub limit: i32,
    #[builder(default = 0)]
    pub offset: i32,
    #[builder(into)]
    pub id: Option<String>,
}
