use cinavault_server::serve;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let bind_address = std::env::var("CINAVAULT_SERVER_BIND")
        .unwrap_or_else(|_| "127.0.0.1:8097".to_owned())
        .parse::<SocketAddr>()
        .expect("CINAVAULT_SERVER_BIND must be a valid socket address");

    eprintln!("CinaVault 3.0 service foundation listening on {bind_address}");
    serve(bind_address).await
}
