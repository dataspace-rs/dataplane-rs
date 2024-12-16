use bon::Builder;
use serde::Deserialize;
use std::net::{IpAddr, Ipv4Addr};

use secrecy::SecretString;

#[derive(Deserialize, Clone, Builder)]
pub struct DataPlaneCfg {
    #[serde(default = "default_db")]
    pub db: Database,
    pub signaling: Signaling,
    pub component_id: String,
    pub proxy: Proxy,
}

#[derive(Deserialize, Clone, Builder)]
pub struct Signaling {
    #[builder(into)]
    pub control_plane_url: String,
    #[builder(into)]
    pub signaling_url: String,
    #[serde(default = "default_signaling_port")]
    pub port: u16,
    #[serde(default = "default_bind")]
    pub bind: IpAddr,
}

#[derive(Deserialize, Clone, Builder)]
pub struct TokenRenewal {
    #[serde(default = "default_renewal_port")]
    pub port: u16,
    #[serde(default = "default_bind")]
    pub bind: IpAddr,
}

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

#[derive(Deserialize, Clone, Debug, PartialEq, Copy)]
pub enum KeyFormat {
    Pem,
}

pub fn default_renewal() -> TokenRenewal {
    TokenRenewal {
        port: default_renewal_port(),
        bind: default_bind(),
    }
}

pub fn default_db() -> Database {
    Database::Sqlite {
        path: ":memory:".to_string(),
    }
}

pub fn default_signaling_port() -> u16 {
    8787
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

#[derive(Deserialize, Clone)]
pub enum Database {
    Sqlite { path: String },
}
