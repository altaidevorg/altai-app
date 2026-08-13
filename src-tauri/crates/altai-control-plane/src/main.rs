use altai_control_plane::{
    router, BootstrapCredential, ControlPlane, ControlPlaneConfig, ControlPlaneStore,
};
use clap::Parser;
use std::{net::SocketAddr, sync::Arc};

#[derive(Parser)]
#[command(
    name = "altai-control-plane",
    about = "ALTAI authenticated control-plane daemon"
)]
struct Args {
    /// Loopback listener. Non-loopback listeners require a future TLS/proxy deployment path.
    #[arg(long, default_value = "127.0.0.1:8787")]
    bind: SocketAddr,
    /// PGlite data directory; the database adapter is introduced in a later slice.
    #[arg(long, default_value = ".altai/control-plane")]
    pglite_dir: String,
    /// Bootstrap bearer credential. Prefer ALTAI_CONTROL_PLANE_BOOTSTRAP_TOKEN.
    #[arg(long, env = "ALTAI_CONTROL_PLANE_BOOTSTRAP_TOKEN")]
    bootstrap_token: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if !args.bind.ip().is_loopback() {
        return Err("control-plane daemon only permits loopback bind in this milestone".into());
    }
    let credential = BootstrapCredential::from_plaintext(&args.bootstrap_token)?;
    let plane = Arc::new(ControlPlane::bootstrap(ControlPlaneConfig {
        service_version: env!("CARGO_PKG_VERSION").to_string(),
        store: ControlPlaneStore::Pglite {
            data_dir: args.pglite_dir,
        },
        registration_ttl_seconds: 300,
    })?);
    let listener = tokio::net::TcpListener::bind(args.bind).await?;
    eprintln!(
        "altai-control-plane listening on {}",
        listener.local_addr()?
    );
    axum::serve(listener, router(plane, credential)).await?;
    Ok(())
}
