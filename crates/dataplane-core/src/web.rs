use std::net::IpAddr;

use axum::Router;
use std::net::TcpStream;
use tokio::{
    net::TcpListener,
    sync::watch::{Receiver, Sender},
};

pub async fn start_server<T: Clone + Send + Sync + 'static>(
    bind: IpAddr,
    port: u16,
    app: Router<T>,
    state: T,
    name: &'static str,
) -> anyhow::Result<ServerHandle> {
    let app = app.with_state(state);
    let listener = TcpListener::bind((bind, port)).await?;
    let server_addr = listener.local_addr()?;

    let (shutdown_trigger, shutdown_receiver) = tokio::sync::watch::channel(());
    let (shutdown_notifier, shutdown_listener) = tokio::sync::watch::channel(());

    tokio::task::spawn(async move {
        tracing::debug!("Launching {} on {}", name, listener.local_addr().unwrap());
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal(shutdown_receiver))
            .await
            .unwrap();

        shutdown_notifier.send(()).unwrap();
    });

    wait_for_server(server_addr).await;

    Ok(ServerHandle::new(shutdown_trigger, shutdown_listener))
}

pub struct ServerHandle {
    shutdown: Sender<()>,
    waiter: Receiver<()>,
}

impl ServerHandle {
    pub fn new(shutdown: Sender<()>, waiter: Receiver<()>) -> Self {
        Self { shutdown, waiter }
    }

    pub async fn shutdown(self) {
        self.shutdown.send(()).unwrap();
    }

    pub async fn wait(&mut self) -> anyhow::Result<()> {
        self.waiter.changed().await.map(Ok)?
    }
}

async fn shutdown_signal(mut receiver: Receiver<()>) {
    let _ = receiver.changed().await;
}

pub async fn wait_for_server(socket: std::net::SocketAddr) {
    for _ in 0..10 {
        if TcpStream::connect_timeout(&socket, std::time::Duration::from_millis(25)).is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
}
