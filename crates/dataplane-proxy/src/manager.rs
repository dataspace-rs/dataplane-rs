use std::str::FromStr;

use axum::async_trait;
use chrono::Duration;
use edc_dataplane_core::{
    core::{
        model::transfer::{types::TransferKind, Transfer},
        service::transfer::TransferManager,
    },
    signaling::DataAddress,
};
use jsonwebtoken::Algorithm;

use crate::{
    config::Proxy,
    service::{
        edr::EdrManager,
        refresh::RefreshManager,
        token::{TokenManager, TokenManagerImpl},
    },
};

pub struct TransferProxyManager<T: TokenManager> {
    edrs: EdrManager<T>,
}

pub fn manager_from_config(proxy: Proxy) -> anyhow::Result<TransferProxyManager<TokenManagerImpl>> {
    let token_manager = create_token_manager(proxy.clone())?;

    let edr_manager = create_edr_manager(token_manager, proxy)?;

    Ok(TransferProxyManager { edrs: edr_manager })
}

fn create_token_manager(proxy: Proxy) -> anyhow::Result<TokenManagerImpl> {
    let proxy_url = proxy
        .proxy_url
        .clone()
        .unwrap_or_else(|| format!("http://localhost:{}/api/v1/public", proxy.port));

    Ok(TokenManagerImpl::builder()
        .encoding_key(proxy.keys.private_key.clone())
        .decoding_key(proxy.keys.public_key)
        .algorithm(Algorithm::from_str(&proxy.keys.algorithm)?)
        .audience(proxy_url)
        .kid(proxy.keys.kid.clone())
        .format(proxy.keys.format)
        .leeway(proxy.token_leeway)
        .build())
}

fn create_edr_manager(
    tokens: TokenManagerImpl,
    proxy: Proxy,
) -> anyhow::Result<EdrManager<TokenManagerImpl>> {
    let token_duration = Duration::seconds(proxy.token_duration as i64);
    let refresh_token_duration = Duration::seconds(proxy.refresh_token_duration as i64);

    let token_url = proxy
        .token_url
        .clone()
        .unwrap_or_else(|| format!("http://localhost:{}/api/v1/token", proxy.renewal.port));

    let jwks_url = proxy.jwks_url.clone().unwrap_or_else(|| {
        format!(
            "http://localhost:{}/.well-known/jwks.json",
            proxy.renewal.port
        )
    });
    Ok(EdrManager::builder()
        .tokens(tokens.clone())
        .proxy_url(tokens.audience().to_string())
        .issuer(proxy.issuer.clone())
        .token_duration(token_duration)
        .refresh_token_duration(refresh_token_duration)
        .token_url(token_url)
        .jwks_url(jwks_url)
        .build())
}

impl<T: TokenManager> TransferProxyManager<T> {
    pub fn new(edrs: EdrManager<T>) -> Self {
        Self { edrs }
    }
}

#[async_trait]
impl<T: TokenManager + Send + Sync + 'static> TransferManager for TransferProxyManager<T> {
    async fn can_handle(&self, transfer: &Transfer) -> anyhow::Result<bool> {
        let _ = TransferKind::try_from(&transfer.source.0)?;

        Ok(true)
    }

    async fn handle_start(&self, transfer: &Transfer) -> anyhow::Result<Option<DataAddress>> {
        self.edrs.create_edr(&transfer).await?;
        todo!()
    }

    async fn handle_suspend(&self, id: &str) -> anyhow::Result<()> {
        // todo handle suspend
        Ok(())
    }
    async fn handle_terminate(&self, id: &str) -> anyhow::Result<()> {
        // todo handle terminate
        Ok(())
    }
}
