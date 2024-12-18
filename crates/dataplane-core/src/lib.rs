mod config;
mod dataplane;
mod registration;
pub mod signaling;
mod web;
pub use dataplane::{DataPlane, DataPlaneHandle};
pub mod core;
mod tracing;

pub mod derive {
    pub use edc_dataplane_macros::interface;
}
pub use config::{
    default_bind, default_db, default_proxy_port, default_refresh_token_duration,
    default_renewal_port, default_signaling_port, default_token_duration, DataPlaneCfg, Database,
    KeyFormat, Proxy, ProxyKeys, Signaling, TokenRenewal,
};
