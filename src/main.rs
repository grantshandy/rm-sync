use std::{
    net::{Ipv4Addr, SocketAddr},
    path::PathBuf,
    sync::Arc,
};

use axum::{routing, Router};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{compression::CompressionLayer, trace::TraceLayer};

use crate::rm::Filesystem;

mod dav;
mod rm;
mod web;

/// A web interface/webdav proxy for the reMarkable tablet
#[derive(argh::FromArgs, Clone)]
struct Args {
    /// the desired port to run the HTTP server on
    #[argh(option, short = 'p', default = "8090")]
    port: u16,

    /// don't broadcast HTTP on 0.0.0.0:<PORT>
    #[argh(switch, short = 'b')]
    no_broadcast: bool,

    /// the path to the reMarkable document directory
    #[argh(option, short = 'd', default = "default_doc_path()")]
    documents: PathBuf,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let args: Args = argh::from_env();

    // parse documents
    let fs = Arc::new(Filesystem::from_path(&args.documents).await);

    let (a, _) = tokio::join![http_server(&args, fs.clone()), fs.auto_reindex()];

    a
}

async fn http_server(args: &Args, state: Arc<Filesystem>) -> color_eyre::Result<()> {
    let app = Router::new()
        .route("/", routing::get(web::root))
        .route("/dav", routing::any(dav::dav))
        .route("/dav/", routing::any(dav::dav))
        .route("/dav/*path", routing::any(dav::dav))
        .fallback(web::fallback)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CompressionLayer::new().gzip(true)),
        )
        .with_state(state);

    let socket = SocketAddr::new(
        match args.no_broadcast {
            false => Ipv4Addr::LOCALHOST.into(),
            true => Ipv4Addr::UNSPECIFIED.into(),
        },
        args.port,
    );

    tracing::info!("Launching {} at http://{}/", env!("CARGO_PKG_NAME"), socket);

    // print a helpful message for broadcasting servers with a line for each ipv4 broadcast interface
    if args.no_broadcast {
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

pub fn is_remarkable() -> bool {
    return cfg!(all(
        target_arch = "arm",
        target_env = "musl",
        target_os = "linux"
    ));
}

/// Try to guess a good default document path based on the OS
pub fn default_doc_path() -> PathBuf {
    if is_remarkable() {
        tracing::debug!("Assuming we're running on the reMarkable tablet");
        "/home/root/.local/share/remarkable/xochitl/".into()
    } else {
        "./samples/v6/".into()
    }
}
