use std::net::IpAddr;

use bon::Builder;
use edc_dataplane_core::config::{default_bind, KeyFormat};
use secrecy::SecretString;
use serde::Deserialize;

#[derive(Deserialize, Clone, Builder)]
pub struct Proxy {
    #[serde(default = "default_proxy_port")]
    pub port: u16,
    #[serde(default = "default_bind")]
    pub bind: IpAddr,
    #[builder(into)]
    pub proxy_url: Option<String>,
    #[builder(into)]
    pub token_url: Option<String>,
    #[builder(into)]
    pub jwks_url: Option<String>,
    #[serde(default = "default_token_duration")]
    pub token_duration: u64,
    #[builder(into)]
    pub issuer: String,
    pub keys: ProxyKeys,
    #[serde(default = "default_refresh_token_duration")]
    pub refresh_token_duration: u64,
    #[serde(default = "default_token_leeway")]
    pub token_leeway: u64,
    #[serde(default = "default_renewal")]
    pub renewal: TokenRenewal,
}

#[derive(Deserialize, Clone, Builder)]
pub struct ProxyKeys {
    pub private_key: SecretString,
    pub public_key: String,
    #[builder(into)]
    pub kid: String,
    #[builder(into)]
    pub algorithm: String,
    pub format: KeyFormat,
}

#[derive(Deserialize, Clone, Builder)]
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
