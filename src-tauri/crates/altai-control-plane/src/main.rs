use altai_control_plane::{
    router, BootstrapCredential, ControlPlane, ControlPlaneConfig, ControlPlaneStore,
    PostgresRegistrationRepository,
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
    /// Deployed Postgres control database. Overrides the embedded PGlite mode.
    #[arg(long, env = "ALTAI_CONTROL_PLANE_POSTGRES_URL")]
    postgres_url: Option<String>,
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
    let store = args
        .postgres_url
        .as_ref()
        .map(|connection_url| ControlPlaneStore::Postgres {
            connection_url: connection_url.clone(),
        })
        .unwrap_or(ControlPlaneStore::Pglite {
            data_dir: args.pglite_dir,
        });
    let config = ControlPlaneConfig {
        service_version: env!("CARGO_PKG_VERSION").to_string(),
        store,
        registration_ttl_seconds: 300,
    };
    let plane = if let Some(connection_url) = args.postgres_url {
        Arc::new(ControlPlane::with_registration_repository(
            config,
            Arc::new(PostgresRegistrationRepository::connect(&connection_url)?),
        )?)
    } else {
        Arc::new(ControlPlane::bootstrap(config)?)
    };
    let listener = tokio::net::TcpListener::bind(args.bind).await?;
    eprintln!(
        "altai-control-plane listening on {}",
        listener.local_addr()?
    );
    axum::serve(listener, router(plane, credential)).await?;
    Ok(())
}
