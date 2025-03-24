use miwa::{
    core::{Extension, ExtensionConfig, MiwaContext, MiwaResult},
    derive::{extension, ExtensionConfig},
};
use serde::Deserialize;

use crate::core::db::{sqlite::transfer::SqliteTransferRepo, transfer::TransferRepoRef};

pub struct SqliteRepoExtension {}

#[async_trait::async_trait]
impl Extension for SqliteRepoExtension {
    async fn start(&self) -> MiwaResult<()> {
        Ok(())
    }

    async fn shutdown(&self) -> MiwaResult<()> {
        Ok(())
    }
}

#[derive(Deserialize, ExtensionConfig)]
#[config(prefix = "db.transfers")]
#[serde(rename_all = "lowercase")]
pub enum TransferDbConfig {
    Sqlite { path: String },
}

#[extension(
    name = "Sqlite store extensions for dataplane",
    provides(TransferRepoRef)
)]
pub async fn sql_repo_extension(
    ctx: &MiwaContext,
    ExtensionConfig(cfg): ExtensionConfig<TransferDbConfig>,
) -> MiwaResult<SqliteRepoExtension> {
    ctx.register(create_transfer_store(cfg).await?);
    Ok(SqliteRepoExtension {})
}

async fn create_transfer_store(cfg: TransferDbConfig) -> anyhow::Result<TransferRepoRef> {
    match cfg {
        TransferDbConfig::Sqlite { path } => {
            let store = SqliteTransferRepo::connect(&format!("sqlite:{}", path)).await?;
            store.migrate().await?;

            Ok(TransferRepoRef::of(store))
        }
    }
}
