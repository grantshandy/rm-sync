use std::{
    net::{Ipv4Addr, SocketAddr},
    path::PathBuf,
};

use axum::{
    extract::{FromRef, State},
    routing, Router,
};
use maud::{html, Markup, PreEscaped, DOCTYPE};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{compression::CompressionLayer, trace::TraceLayer};

use crate::rm::Filesystem;

mod rm;
mod webdav;

/// A web interface/webdav proxy for ReMarkable
#[derive(argh::FromArgs, Clone)]
struct Args {
    /// which port to launch the HTTP server on
    #[argh(option, short = 'p', default = "8090")]
    port: u16,

    /// broadcast to the local network (use on ReMarkable)
    #[argh(switch)]
    broadcast: bool,

    /// the path to the remarkable document directory
    #[argh(option, short = 'd', default = "rm::default_doc_path()")]
    documents: PathBuf,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let args: Args = argh::from_env();

    // parse documents
    let fs = Filesystem::from_path(&args.documents);
    tracing::debug!("{:#?}", fs.list("/Books"));

    http_server(
        &args,
        AppState {
            fs,
        },
    )
    .await?;
    Ok(())
}

#[derive(Clone, FromRef)]
struct AppState {
    fs: Filesystem,
}

async fn http_server(args: &Args, state: AppState) -> color_eyre::Result<()> {
    let app = Router::new()
        .route("/", routing::get(root))
        .route("/dav", routing::any(webdav::dav))
        .route("/dav/", routing::any(webdav::dav))
        .route("/dav/*path", routing::any(webdav::dav))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CompressionLayer::new().gzip(true)),
        )
        .with_state(state);

    let socket = SocketAddr::new(
        match args.broadcast {
            false => Ipv4Addr::LOCALHOST.into(),
            true => Ipv4Addr::UNSPECIFIED.into(),
        },
        args.port,
    );

    tracing::info!("Launching {} at http://{}/", env!("CARGO_PKG_NAME"), socket);

    // print a helpful message for broadcasting servers with a line for each ipv4 broadcast interface
    if args.broadcast {
        for ip in pnet::datalink::interfaces()
            .iter()
            .filter(|interface| interface.is_broadcast())
            .flat_map(|interface| &interface.ips)
            .filter(|&ip| ip.is_ipv4())
            .map(|ip| ip.ip())
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

async fn root(State(fs): State<Filesystem>) -> Markup {
    page(
        "rm-cloudsync",
        html! {
            h1 { ("rm-cloudsync") }
            pre { (PreEscaped(format!("{fs:#?}"))) }
        },
    )
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
