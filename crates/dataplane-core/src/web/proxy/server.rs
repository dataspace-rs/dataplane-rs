use std::{net::SocketAddr, thread};

use pingora::server::{configuration::ServerConf, Server};
use pingora_proxy::http_proxy_service;

use crate::{
    core::service::token::TokenManager,
    web::{state::Context, util::wait_for_server},
    Proxy,
};

use super::public::PublicProxy;

pub async fn start<T: TokenManager + Send + Sync + Clone + 'static>(cfg: &Proxy, ctx: Context<T>) {
    let mut server = Server::new_with_opt_and_conf(None, ServerConf::default());
    let addr = format!("{}:{}", cfg.bind, cfg.port);
    server.bootstrap();

    let mut proxy = http_proxy_service(&server.configuration, PublicProxy::new(ctx));

    proxy.add_tcp(&addr);
    server.add_service(proxy);

    let srv_addr = addr.parse::<SocketAddr>().unwrap();
    thread::spawn(move || {
        tracing::debug!("Launching proxy on {}", addr);
        server.run_forever();
    });

    wait_for_server(srv_addr).await;
}
