use std::{
    net::{Ipv4Addr, SocketAddr},
    path::PathBuf,
};

use axum::{extract::Path, routing, Router};
use maud::{html, Markup, PreEscaped, DOCTYPE};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{compression::CompressionLayer, trace::TraceLayer};

mod rm;

/// A web interface/webdav proxy for ReMarkable
#[derive(argh::FromArgs, Clone)]
struct Args {
    /// which port to launch the HTTP server on
    #[argh(option, short = 'p', default = "8090")]
    port: u16,

    /// don't broadcast on 0.0.0.0 (helpful for non-root users and development)
    #[argh(switch)]
    no_broadcast: bool,

    /// the path to the remarkable document directory
    #[argh(
        option,
        short = 'd',
        default = r#""/home/root/.local/xochitl/".into()"#
    )]
    documents: PathBuf,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let args: Args = argh::from_env();

    // parse documents
    let _doc = rm::Documents::from_path(&args.documents).await?;

    http_server(&args).await?;
    Ok(())
}

async fn http_server(args: &Args) -> color_eyre::Result<()> {
    let app = Router::new()
        .route("/", routing::get(root))
        .route("/button/:count", routing::get(button))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CompressionLayer::new().gzip(true)),
        );

    let socket = SocketAddr::new(
        match args.no_broadcast {
            true => Ipv4Addr::LOCALHOST.into(),
            false => Ipv4Addr::UNSPECIFIED.into(),
        },
        args.port,
    );

    tracing::info!("Launching {} at http://{}/", env!("CARGO_PKG_NAME"), socket);
    if !args.no_broadcast {
        for ip in pnet::datalink::interfaces()
            .iter()
            .filter(|interface| interface.is_broadcast())
            .flat_map(|interface| &interface.ips)
            .filter_map(|ip| ip.is_ipv4().then(|| ip.ip()))
        {
            tracing::info!(
                "The server may be available at http://{}:{}/",
                ip,
                args.port
            );
        }
    }

    axum::serve(TcpListener::bind(&socket).await?, app).await?;
    Ok(())
}

async fn root() -> Markup {
    page(
        "rm-cloudsync",
        html! {
            h1 { ("rm-cloudsync") }
            (button(Path(0)).await)
        },
    )
}

async fn button(Path(count): Path<i32>) -> Markup {
    html! {
        p id="button" hx-swap="outerHTML" hx-target="#button" {
            button hx-get={("/button/")(count - 1)} {("-")}
            (' ')(count)(' ')
            button hx-get={("/button/")(count + 1)} {("+")}
        }
    }
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
