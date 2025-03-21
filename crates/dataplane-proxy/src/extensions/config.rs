use std::net::{IpAddr, Ipv4Addr};

use miwa::derive::ExtensionConfig;
use secrecy::SecretString;
use serde::Deserialize;

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

#[derive(Deserialize, Clone, Debug, PartialEq, Copy)]
pub enum KeyFormat {
    Pem,
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

pub fn default_bind() -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0))
}
