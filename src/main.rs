use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use axum::{response::IntoResponse, routing, Router};
use maud::{html, Markup, PreEscaped, DOCTYPE};
use tokio::net::TcpListener;

const PORT: u16 = 3300;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    // log all outgoing ports on this network we're serving on for remarkable user's convenience
    for ip in pnet::datalink::interfaces()
        .iter()
        .map(|interface| &interface.ips)
        .flatten()
        .filter(|ip| ip.is_ipv4())
        .map(|ip| ip.ip())
    {
        tracing::info!("Launching web server at http://{ip}:{PORT}/");
    }

    let app = Router::new()
        .route("/", routing::get(root))
        .route("/clicked", routing::get(time_button));

    let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), PORT);
    let listener = TcpListener::bind(&socket).await?;

    axum::serve(listener, app).await?;

    Ok(())
}

async fn root() -> impl IntoResponse {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="UTF-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                script { (PreEscaped(include_str!(env!("HTMX")))) }
                title { ("rm-cloudsync") }
            }
            body {
                h1 { ("rm-cloudsync :)") }
                (time_button().await)
            }
        }
    }
}

async fn time_button() -> Markup {
    html! {
        button hx-get="/clicked" hx-swap="outerHTML" {
            (time::OffsetDateTime::now_utc())
        }
    }
}
