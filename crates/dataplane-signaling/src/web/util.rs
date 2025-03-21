use std::net::TcpStream;

pub async fn wait_for_server(socket: std::net::SocketAddr) {
    for _ in 0..10 {
        if TcpStream::connect_timeout(&socket, std::time::Duration::from_millis(25)).is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
}
