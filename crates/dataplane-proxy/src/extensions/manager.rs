use std::net::IpAddr;
use std::str::FromStr;

use chrono::Duration;
use edc_dataplane_core::{config::KeyFormat, core::service::transfer::TransferManagerRef};
 use edc_dataplane_core::config::default_bind;
use jsonwebtoken::Algorithm;
use miwa::core::ExtensionConfig;
use miwa::derive::ExtensionConfig;
use miwa::{core::{Extension, MiwaContext, MiwaResult}, derive::extension};
use secrecy::SecretString;
use serde::Deserialize;

use crate::service::edr::EdrManager;
use crate::{manager::TransferProxyManager, service::token::TokenManagerImpl};

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
pub async fn transfer_proxy_extension(ctx: &MiwaContext, ExtensionConfig(cfg): ExtensionConfig<Proxy>) -> MiwaResult<TransferManagerExtension> {
    ctx.register(TransferManagerRef::of(manager_from_config(cfg)?));
    Ok(TransferManagerExtension)
}


pub fn manager_from_config(proxy: Proxy) -> anyhow::Result<TransferProxyManager<TokenManagerImpl>> {
    let token_manager = create_token_manager(proxy.clone())?;

    let edr_manager = create_edr_manager(token_manager, proxy)?;

    Ok(TransferProxyManager::new(edr_manager))
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

#[derive(Deserialize, Clone, ExtensionConfig)]
#[config(prefix = "proxy")]
pub struct Proxy {
    #[serde(default = "default_proxy_port")]
    pub port: u16,
    #[serde(default = "default_bind")]
    pub bind: IpAddr,
    pub proxy_url: Option<String>,
    pub token_url: Option<String>,
    pub jwks_url: Option<String>,
    #[serde(default = "default_token_duration")]
    pub token_duration: u64,
    pub issuer: String,
    pub keys: ProxyKeys,
    #[serde(default = "default_refresh_token_duration")]
    pub refresh_token_duration: u64,
    #[serde(default = "default_token_leeway")]
    pub token_leeway: u64,
    #[serde(default = "default_renewal")]
    pub renewal: TokenRenewal,
}

#[derive(Deserialize, Clone)]
pub struct ProxyKeys {
    pub private_key: SecretString,
    pub public_key: String,
    pub kid: String,
    pub algorithm: String,
    pub format: KeyFormat,
}

#[derive(Deserialize, Clone)]
pub struct TokenRenewal {
    #[serde(default = "default_renewal_port")]
    pub port: u16,
    #[serde(default = "default_bind")]
    pub bind: IpAddr,
}

pub fn default_renewal() -> TokenRenewal {
    TokenRenewal {
        port: default_renewal_port(),
        bind: default_bind(),
    }
}

pub fn default_renewal_port() -> u16 {
    8788
}

pub fn default_proxy_port() -> u16 {
    8789
}
pub fn default_token_duration() -> u64 {
    60 * 10
}

pub fn default_refresh_token_duration() -> u64 {
    60 * 60 * 24 * 30
}

pub fn default_token_leeway() -> u64 {
    60
}
