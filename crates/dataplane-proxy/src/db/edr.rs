use async_trait::async_trait;
use miwa::derive::interface;

#[cfg(test)]
use mockall::{automock, predicate::*};

use crate::model::edr::EdrEntry;

#[async_trait]
#[interface]
#[cfg_attr(test, automock)]
pub trait EdrRepo {
    async fn save(&self, edr: EdrEntry) -> anyhow::Result<()>;
    async fn fetch_by_id(&self, transfer_id: &str) -> anyhow::Result<Option<EdrEntry>>;
    async fn delete(&self, transfer_id: &str) -> anyhow::Result<()>;
}
