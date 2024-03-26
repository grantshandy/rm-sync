use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use axum::{routing, Router};
use maud::{html, Markup, PreEscaped, DOCTYPE};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{compression::CompressionLayer, trace::TraceLayer};

pub const PORT: u16 = 3300;

pub async fn server() -> color_eyre::Result<()> {
    let app = Router::new()
        .route("/", routing::get(root))
        .route("/time", routing::get(time))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CompressionLayer::new().gzip(true)),
        );

    let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), PORT);
    let listener = TcpListener::bind(&socket).await?;

    axum::serve(listener, app).await?;

    Ok(())
}

async fn root() -> Markup {
    page(
        "rm-cloudsync",
        html! {
            h1 { ("rm-cloudsync") }
            button hx-get="/time" hx-swap="innerHTML" { (time().await) }
        },
    )
}

async fn time() -> Markup {
    html! { (time::OffsetDateTime::now_utc()) }
}

fn page(title: impl AsRef<str>, body: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="UTF-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                script { (PreEscaped(include_str!(env!("HTMX")))) }
                title { (title.as_ref()) }
            }
            body { (body) }
        }
    }
}
