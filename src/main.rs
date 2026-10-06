use crate::server::SERVER_CONTEXT;
use clap::Parser;
use log::{LevelFilter, info};
use std::net::SocketAddr;
use tokio::net::TcpListener;

mod auth;
mod data;
mod routes;
mod server;
mod utils;

/// Server-side sync engine for the LX Music ecosystem.
///
/// Real-time playlist and dislike list synchronization across devices via WebSocket.
#[derive(Parser)]
#[command(version)]
struct Cli {
    /// Enable debug logging level (ignores RUST_LOG).
    #[arg(long)]
    debug: bool,

    /// Enable trace logging level (ignores RUST_LOG).
    #[arg(long)]
    trace: bool,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let cli = Cli::parse();

    const VERSION: &str = env!("CARGO_PKG_VERSION");
    const LOG_LEVEL: &str = if cfg!(debug_assertions) {
        "debug"
    } else {
        "info"
    };
    let mut log_builder = env_logger::Builder::new();
    log_builder.target(env_logger::Target::Stdout);
    if cli.trace {
        log_builder.filter_level(LevelFilter::Trace);
    } else if cli.debug {
        log_builder.filter_level(LevelFilter::Debug);
    } else {
        log_builder.parse_env(env_logger::Env::default().default_filter_or(LOG_LEVEL));
    }
    log_builder.init();
    info!("Welcome to LX Music Sync Server(rs) {VERSION}");

    SERVER_CONTEXT.start_daemon();

    // build the application router
    let app = routes::app().into_make_service_with_connect_info::<SocketAddr>();

    let addr = SocketAddr::from(([127, 0, 0, 1], 9527));
    let listener = TcpListener::bind(addr).await?;
    info!("Listening on {addr}.");
    axum::serve(listener, app).await?;
    Ok(())
}
