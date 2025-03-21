use miwa::{
    core::{Extension, ExtensionConfig, MiwaContext, MiwaResult},
    derive::{extension, ExtensionConfig},
};
use serde::Deserialize;

use crate::db::{edr::EdrRepoRef, sqlite::edr::SqliteEdrRepo};

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
#[config(prefix = "db.tokens")]
#[serde(rename_all = "lowercase")]
pub enum TokenDbConfig {
    Sqlite { path: String },
}

#[extension(
    name = "Sqlite store extensions for dataplane proxy",
    provides(EdrRepoRef)
)]
pub async fn proxy_sql_repo_extension(
    ctx: &MiwaContext,
    ExtensionConfig(cfg): ExtensionConfig<TokenDbConfig>,
) -> MiwaResult<SqliteRepoExtension> {
    ctx.register(create_token_store(cfg).await?);
    Ok(SqliteRepoExtension {})
}

async fn create_token_store(cfg: TokenDbConfig) -> anyhow::Result<EdrRepoRef> {
    match cfg {
        TokenDbConfig::Sqlite { path } => {
            let store = SqliteEdrRepo::connect(&format!("sqlite:{}", path)).await?;
            store.migrate().await?;

            Ok(EdrRepoRef::of(store))
        }
    }
}
