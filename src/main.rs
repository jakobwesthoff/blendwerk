/*
 * Copyright (c) 2025 Jakob Westhoff <jakob@westhoffswelt.de>
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod frontmatter;
mod routes;
mod server;
mod tls;
mod watcher;

use clap::{Parser, ValueEnum};
use pid1::Pid1Settings;
use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, watch};
use tracing::{error, info};

#[derive(Debug, Clone, ValueEnum)]
enum CertMode {
    /// No HTTPS, HTTP only
    None,
    /// Generate self-signed certificate on startup
    SelfSigned,
    /// Use custom certificate files
    Custom,
}

#[derive(Parser, Debug)]
#[command(name = "blendwerk")]
#[command(about = "A file-based mock HTTP/HTTPS server for testing")]
#[command(version)]
#[command(author)]
struct Args {
    /// Directory containing mock responses
    directory: PathBuf,

    /// HTTP port
    #[arg(short = 'p', long, default_value = "8080")]
    http_port: u16,

    /// HTTPS port
    #[arg(short = 's', long, default_value = "8443")]
    https_port: u16,

    /// Only serve HTTP (no HTTPS)
    #[arg(long, conflicts_with = "https_only")]
    http_only: bool,

    /// Only serve HTTPS (no HTTP)
    #[arg(long, conflicts_with = "http_only")]
    https_only: bool,

    /// Certificate mode
    #[arg(long, value_enum, default_value = "self-signed")]
    cert_mode: CertMode,

    /// Path to certificate file (required for custom cert mode)
    #[arg(long, required_if_eq("cert_mode", "custom"))]
    cert_file: Option<PathBuf>,

    /// Path to private key file (required for custom cert mode)
    #[arg(long, required_if_eq("cert_mode", "custom"))]
    key_file: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    // Set up pid1 handler if running as PID 1 (e.g., in containers)
    Pid1Settings::new()
        .enable_log(true)
        .timeout(Duration::from_secs(5))
        .launch()?;

    main_inner()
}

#[tokio::main]
async fn main_inner() -> anyhow::Result<()> {
    // Initialize tracing subscriber for request logging
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    let args = Args::parse();

    // Validate directory exists
    if !args.directory.exists() {
        anyhow::bail!("Directory '{}' does not exist", args.directory.display());
    }

    if !args.directory.is_dir() {
        anyhow::bail!("'{}' is not a directory", args.directory.display());
    }

    info!("Starting blendwerk...");
    info!("  Directory: {}", args.directory.display());
    info!("  HTTP port: {}", args.http_port);
    info!("  HTTPS port: {}", args.https_port);
    info!("  Cert mode: {:?}", args.cert_mode);

    let run_http = !args.https_only;
    let run_https = !args.http_only && !matches!(args.cert_mode, CertMode::None);

    if run_http && run_https {
        info!("  Mode: HTTP and HTTPS");
    } else if run_http {
        info!("  Mode: HTTP only");
    } else if run_https {
        info!("  Mode: HTTPS only");
    } else {
        anyhow::bail!("No server to run (both HTTP and HTTPS disabled)");
    }

    // Scan directory for routes
    let routes = routes::scan_directory(&args.directory)?;
    info!("  Loaded {} routes", routes.len());

    for route in &routes {
        info!("    {:?} {}", route.method, route.display_path());
    }

    // Create shared routes for hot-reload
    let shared_routes = Arc::new(RwLock::new(routes));

    // Create shutdown signal
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Set up signal handler for SIGTERM and SIGINT
    let signal_tx = shutdown_tx.clone();
    let mut signals = Signals::new([SIGTERM, SIGINT])?;
    std::thread::spawn(move || {
        for sig in signals.forever() {
            match sig {
                SIGTERM => info!("Received SIGTERM, shutting down..."),
                SIGINT => info!("Received SIGINT, shutting down..."),
                _ => {}
            }
            let _ = signal_tx.send(true);
            break;
        }
    });

    // Get TLS config if needed
    let tls_config = if run_https {
        Some(match args.cert_mode {
            CertMode::SelfSigned => {
                info!("  Generating self-signed certificate...");
                tls::create_self_signed_config().await?
            }
            CertMode::Custom => {
                let cert_file = args.cert_file.as_ref().unwrap();
                let key_file = args.key_file.as_ref().unwrap();
                info!(
                    "  Loading certificate from {} and {}",
                    cert_file.display(),
                    key_file.display()
                );
                tls::load_custom_config(cert_file, key_file).await?
            }
            CertMode::None => unreachable!(),
        })
    } else {
        None
    };

    // Spawn file watcher for hot-reload
    let watcher_routes = shared_routes.clone();
    let watcher_dir = args.directory.clone();
    let watcher_shutdown = shutdown_rx.clone();
    tokio::spawn(async move {
        if let Err(e) =
            watcher::watch_directory(watcher_dir, watcher_routes, watcher_shutdown).await
        {
            error!("Watcher error: {}", e);
        }
    });

    // Spawn servers
    let mut handles = vec![];

    if run_http {
        let routes = shared_routes.clone();
        let shutdown = shutdown_rx.clone();
        let port = args.http_port;
        handles.push(tokio::spawn(async move {
            server::run_http_server(routes, port, shutdown).await
        }));
    }

    if run_https {
        let routes = shared_routes.clone();
        let shutdown = shutdown_rx.clone();
        let port = args.https_port;
        let tls = tls_config.unwrap();
        handles.push(tokio::spawn(async move {
            server::run_https_server(routes, port, tls, shutdown).await
        }));
    }

    // Wait for servers to finish (they'll stop when shutdown signal is sent)
    for handle in handles {
        let _ = handle.await;
    }

    Ok(())
}
