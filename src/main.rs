//! One executable, many roles (ADR 0028): `kamosu` with no arguments serves;
//! terminal commands are arguments to the same binary, because a distroless image
//! has no shell — `docker exec kamosu /app/kamosu status`, never a pipeline.

use std::net::SocketAddr;
use std::sync::Arc;

use clap::{Parser, Subcommand};
use serde_json::json;

use kamosu::config::Config;
use kamosu::core::Core;
use kamosu::{db, design_tokens, http_min, mcp_door, web_door};

#[derive(Parser)]
#[command(
    name = "kamosu",
    about = "A self-hosted cookbook where people and agents are equal users.",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Serve Kamosu on the configured address and port (the default).
    Serve,
    /// Print this instance's status as JSON: the version and whether setup has happened.
    Status,
    /// Check that this Kamosu answers its own Operations. Exits 0 when healthy.
    /// Run by the container's HEALTHCHECK — a distroless image has no curl to do it.
    HealthCheck,
}

fn main() {
    let cli = Cli::parse();
    let config = Config::from_env();

    match cli.command.unwrap_or(Command::Serve) {
        Command::Serve => serve(config),
        Command::Status => status(&config),
        Command::HealthCheck => {
            let addr: SocketAddr = format!("127.0.0.1:{}", config.port)
                .parse()
                .expect("valid address");
            std::process::exit(health_check(addr));
        }
    }
}

fn serve(config: Config) {
    let filter = tracing_subscriber::EnvFilter::try_new(&config.log_filter)
        .expect("KAMOSU_LOG must be a valid tracing filter, e.g. 'info'");
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    runtime.block_on(async move {
        // Core::start opens the database and starts the Job lanes: slow work is
        // carried from the moment the instance serves.
        let core = Core::start(Arc::new(
            db::Db::open(&config.data_dir).unwrap_or_else(|e| panic!("cannot open /data: {e}")),
        ));
        // The Doors carry the Catalogue; the design tokens ride beside them —
        // static assets and the /tokens dev page, never Operations.
        let app = web_door::router(core.clone())
            .merge(mcp_door::router(core))
            .merge(design_tokens::router());

        let listener = tokio::net::TcpListener::bind(config.bind_address())
            .await
            .unwrap_or_else(|e| panic!("cannot bind {}: {e}", config.bind_address()));
        println!(
            "Kamosu {} serving on http://{} (data in {})",
            env!("CARGO_PKG_VERSION"),
            config.bind_address(),
            config.data_dir.display()
        );

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .expect("server error");
    });
}

async fn shutdown_signal() {
    use tokio::signal::unix::{SignalKind, signal};
    let ctrl_c = tokio::signal::ctrl_c();
    let mut term = signal(SignalKind::terminate()).expect("SIGTERM handler");
    tokio::select! {
        _ = ctrl_c => {},
        _ = term.recv() => {},
    }
}

/// The terminal form of `instance_status`: the same truth, no Door involved.
fn status(config: &Config) {
    let core = Core::open(Arc::new(db::Db::open(&config.data_dir).expect("database")));
    let setup = core.setup_complete().expect("setup flag");
    println!(
        "{}",
        json!({ "version": env!("CARGO_PKG_VERSION"), "setup_complete": setup })
    );
}

/// Ask this Kamosu whether it can carry out its own Operations. One request to
/// one Operation through the real web door; anything but HTTP 200 fails the check.
fn health_check(addr: SocketAddr) -> i32 {
    match http_min::post_json(addr, "/api/op/instance_status", None, "{}") {
        Ok(response) if response.status == 200 => {
            println!("healthy");
            0
        }
        Ok(response) => {
            eprintln!("unhealthy: instance_status answered {}", response.status);
            1
        }
        Err(e) => {
            eprintln!("unhealthy: cannot reach {addr}: {e}");
            1
        }
    }
}
