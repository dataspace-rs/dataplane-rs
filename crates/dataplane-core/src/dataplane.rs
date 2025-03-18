use chrono::Duration;
use config::{Config, Environment, File};
use jsonwebtoken::Algorithm;
use std::{path::PathBuf, str::FromStr};

use crate::{
    config::Database,
    core::{
        db::{sqlx::transfer::sqlite::SqliteTransferStore, transfer::TransferStoreRef},
        service::{
            edr::EdrManager, refresh::RefreshManager, token::TokenManagerImpl,
            transfer::TransferManager,
        },
    },
    registration::register_dataplane,
    web::{self, server::ServerHandle, state::Context},
    DataPlaneCfg,
};

pub struct DataPlane {
    cfg: DataPlaneCfg,
}

pub struct DataPlaneHandle {
    id: String,
    signaling_server: ServerHandle,
    token_server: ServerHandle,
}

impl DataPlaneHandle {
    pub async fn shutdown(self) {
        self.signaling_server.shutdown().await;
        self.token_server.shutdown().await;
    }

    pub async fn wait(&mut self) -> anyhow::Result<()> {
        self.signaling_server.wait().await?;
        self.token_server.wait().await
    }

    pub fn id(&self) -> &str {
        &self.id
    }
}

pub struct DataPlaneBuilder {
    cfg: DataPlaneCfgKind,
}

#[allow(clippy::large_enum_variant)]
pub enum DataPlaneCfgKind {
    File(Option<PathBuf>),
    Input(DataPlaneCfg),
}

impl DataPlane {
    pub fn builder() -> DataPlaneBuilder {
        DataPlaneBuilder {
            cfg: DataPlaneCfgKind::File(None),
        }
    }

    pub async fn start(self) -> anyhow::Result<DataPlaneHandle> {
        let ctx = self.create_context().await?;
        let cfg = self.cfg.clone();

        tokio::task::spawn(async move { register_dataplane(cfg).await });

        let signaling_server = web::server::start(
            self.cfg.signaling.bind,
            self.cfg.signaling.port,
            web::signaling_app(),
            ctx.clone(),
            "signaling",
        )
        .await?;

        let token_server = web::server::start(
            self.cfg.proxy.renewal.bind,
            self.cfg.proxy.renewal.port,
            web::token_app(),
            ctx.clone(),
            "token renewal",
        )
        .await?;

        web::proxy::server::start(&self.cfg.proxy, ctx).await;

        Ok(DataPlaneHandle {
            id: self.cfg.component_id.clone(),
            signaling_server,
            token_server,
        })
    }

    async fn create_context(&self) -> Result<Context<TokenManagerImpl>, anyhow::Error> {
        let token_manager = self.create_token_manager()?;
        let edr_manager = self.create_edr_manager(&token_manager)?;
        let store = self.create_transfer_store().await?;

        let transfer_manager = TransferManager::new(edr_manager.clone(), store.clone());
        let refresh_manager = RefreshManager::new(edr_manager, store);

        let ctx = Context::new(
            transfer_manager,
            token_manager,
            refresh_manager,
        );
        Ok(ctx)
    }
    fn create_token_manager(&self) -> anyhow::Result<TokenManagerImpl> {
        let proxy_url = self.cfg.proxy.proxy_url.clone().unwrap_or_else(|| {
            format!("http://localhost:{}/api/v1/public", self.cfg.signaling.port)
        });

        Ok(TokenManagerImpl::builder()
            .encoding_key(self.cfg.proxy.keys.private_key.clone())
            .decoding_key(&self.cfg.proxy.keys.public_key)
            .algorithm(Algorithm::from_str(&self.cfg.proxy.keys.algorithm)?)
            .audience(proxy_url)
            .kid(self.cfg.proxy.keys.kid.clone())
            .format(self.cfg.proxy.keys.format)
            .leeway(self.cfg.proxy.token_leeway)
            .build())
    }

    fn create_edr_manager(
        &self,
        tokens: &TokenManagerImpl,
    ) -> anyhow::Result<EdrManager<TokenManagerImpl>> {
        let token_duration = Duration::seconds(self.cfg.proxy.token_duration as i64);
        let refresh_token_duration =
            Duration::seconds(self.cfg.proxy.refresh_token_duration as i64);

        let token_url = self.cfg.proxy.token_url.clone().unwrap_or_else(|| {
            format!(
                "http://localhost:{}/api/v1/token",
                self.cfg.proxy.renewal.port
            )
        });

        let jwks_url = self.cfg.proxy.jwks_url.clone().unwrap_or_else(|| {
            format!(
                "http://localhost:{}/.well-known/jwks.json",
                self.cfg.proxy.renewal.port
            )
        });
        Ok(EdrManager::builder()
            .tokens(tokens.clone())
            .proxy_url(tokens.audience().to_string())
            .issuer(self.cfg.proxy.issuer.clone())
            .token_duration(token_duration)
            .refresh_token_duration(refresh_token_duration)
            .token_url(token_url)
            .jwks_url(jwks_url)
            .build())
    }

    async fn create_transfer_store(&self) -> anyhow::Result<TransferStoreRef> {
        match &self.cfg.db {
            Database::Sqlite { path } => {
                let store = SqliteTransferStore::connect(&format!("sqlite:{}", path)).await?;
                store.migrate().await?;

                Ok(TransferStoreRef::of(store))
            }
        }
    }
}

impl DataPlaneBuilder {
    pub fn with_config_file(mut self, cfg: Option<String>) -> DataPlaneBuilder {
        self.cfg = DataPlaneCfgKind::File(cfg.map(PathBuf::from));

        self
    }

    pub fn with_config(mut self, cfg: DataPlaneCfg) -> DataPlaneBuilder {
        self.cfg = DataPlaneCfgKind::Input(cfg);

        self
    }

    fn load_config(&self, path: Option<&PathBuf>) -> anyhow::Result<DataPlaneCfg> {
        let mut config_buider = Config::builder();
        if let Some(path) = path {
            config_buider = config_buider.add_source(File::from(path.clone()));
        }

        config_buider
            .add_source(Environment::with_prefix("dataplane"))
            .build()?
            .try_deserialize()
            .map(Ok)?
    }

    pub fn prepare(self) -> anyhow::Result<DataPlane> {
        let cfg = match self.cfg {
            DataPlaneCfgKind::File(ref cfg) => self.load_config(cfg.as_ref())?,
            DataPlaneCfgKind::Input(cfg) => cfg,
        };

        Ok(DataPlane { cfg })
    }
}
