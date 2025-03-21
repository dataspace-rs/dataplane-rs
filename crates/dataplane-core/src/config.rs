use bon::Builder;
use serde::Deserialize;
use std::net::{IpAddr, Ipv4Addr};

#[derive(Deserialize, Clone, Builder)]
pub struct DataPlaneCfg {
    #[serde(default = "default_db")]
    pub db: Database,
    pub signaling: Signaling,
    pub component_id: String,
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

#[derive(Deserialize, Clone, Debug, PartialEq, Copy)]
pub enum KeyFormat {
    Pem,
}

pub fn default_db() -> Database {
    Database::Sqlite {
        path: ":memory:".to_string(),
    }
}

pub fn default_signaling_port() -> u16 {
    8787
}

pub fn default_bind() -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0))
}

#[derive(Deserialize, Clone)]
pub enum Database {
    Sqlite { path: String },
}
