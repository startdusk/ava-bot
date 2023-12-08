use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use axum_server::tls_rustls::RustlsConfig;
use clap::Parser;
use std::{net::SocketAddr, sync::Arc};
use tower_http::services::ServeDir;
use tracing::info;

use ava_bot::handlers::{assistant_handler, chats_handlers, index_page};

#[derive(Debug, Parser)]
#[clap(name = "ava")]
pub(crate) struct Args {
    #[clap(short, long, default_value = "8080")]
    port: u16,

    #[clap(short, long, default_value = "./.certs")]
    cert_path: String,
}

#[derive(Debug, Default)]
pub(crate) struct AppState {}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let state = Arc::new(AppState::default());
    let app = Router::new()
        .route("/", get(index_page))
        .route("/chats", get(chats_handlers))
        .route("/assistant", post(assistant_handler))
        .nest_service("/public", ServeDir::new("./public"))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], args.port));
    info!("Listening on {}", addr);
    let cert = format!("{}/cert.pem", args.cert_path);
    let key = format!("{}/key.pem", args.cert_path);
    let config = RustlsConfig::from_pem_file(cert, key).await?;
    axum_server::bind_rustls(addr, config)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
