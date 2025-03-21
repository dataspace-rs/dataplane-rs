use chrono::Duration;
use edc_dataplane_core::core::service::transfer::TransferManagerRef;
use jsonwebtoken::Algorithm;
use miwa::core::ExtensionConfig;
use miwa::{
    core::{Extension, MiwaContext, MiwaResult},
    derive::extension,
};
use std::str::FromStr;

use crate::db::edr::EdrRepoRef;
use crate::service::edr::EdrManager;
use crate::{manager::TransferProxyManager, service::token::TokenManagerImpl};

use super::config::Proxy;

pub struct TransferManagerExtension;

#[async_trait::async_trait]
impl Extension for TransferManagerExtension {
    async fn start(&self) -> MiwaResult<()> {
        Ok(())
    }

    async fn shutdown(&self) -> MiwaResult<()> {
        Ok(())
    }
}

#[extension(name = "Transfer Pull manager extension", provides(TransferManagerRef))]
pub async fn transfer_proxy_extension(
    ctx: &MiwaContext,
    ExtensionConfig(cfg): ExtensionConfig<Proxy>,
    edrs: EdrRepoRef,
) -> MiwaResult<TransferManagerExtension> {
    ctx.register(TransferManagerRef::of(manager_from_config(cfg, edrs)?));
    Ok(TransferManagerExtension)
}

pub fn manager_from_config(
    proxy: Proxy,
    edrs: EdrRepoRef,
) -> anyhow::Result<TransferProxyManager<TokenManagerImpl>> {
    let token_manager = create_token_manager(proxy.clone())?;

    let edr_manager = create_edr_manager(edrs.clone(), token_manager, proxy)?;

    Ok(TransferProxyManager::new(edr_manager, edrs))
}

pub fn create_token_manager(proxy: Proxy) -> anyhow::Result<TokenManagerImpl> {
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

pub fn create_edr_manager(
    edrs: EdrRepoRef,
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
        .store(edrs)
        .build())
}
