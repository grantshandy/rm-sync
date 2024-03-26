use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use axum::{routing, Router};
use tokio::net::TcpListener;


#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/", routing::get(root));

    let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 3300);
    let listener = TcpListener::bind(&socket).await?;

    tracing::info!("Launching at http://{socket}");

    axum::serve(listener, app).await?;

    Ok(())
}

async fn root() -> &'static str {
    return "hewwo";
}
