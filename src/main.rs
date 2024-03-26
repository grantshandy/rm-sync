mod http;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    // log all outgoing ports on this network we're serving on for user's convenience
    for ip in pnet::datalink::interfaces()
        .iter()
        .map(|interface| &interface.ips)
        .flatten()
        .filter_map(|ip| ip.is_ipv4().then(|| ip.ip()))
    {
        tracing::info!("Launching http server at http://{}:{}/", ip, http::PORT);
        // tracing::info!("Launching ftp server at ftp://{}:{}/", ip, ftp::PORT)
    }

    tokio::try_join![http::server()]?;

    Ok(())
}