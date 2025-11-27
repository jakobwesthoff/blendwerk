/*
 * Copyright (c) 2025 Jakob Westhoff <jakob@westhoffswelt.de>
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::routes::{HttpMethod, Route};
use axum::{
    Router,
    body::Body,
    extract::State,
    http::{HeaderName, HeaderValue, Method, Request, StatusCode},
    response::Response,
    routing::any,
};
use axum_server::{Handle, tls_rustls::RustlsConfig};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::{RwLock, watch};
use tokio::time::sleep;
use tower_http::trace::{self, TraceLayer};
use tracing::{Level, info};

pub type SharedRoutes = Arc<RwLock<Vec<Route>>>;
pub type ShutdownSignal = watch::Receiver<bool>;

pub struct AppState {
    pub routes: SharedRoutes,
}

fn create_router(routes: SharedRoutes) -> Router {
    let state = Arc::new(AppState { routes });

    Router::new()
        .route("/{*path}", any(handler))
        .route("/", any(handler))
        .with_state(state)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                .on_response(trace::DefaultOnResponse::new().level(Level::INFO)),
        )
}

pub async fn run_http_server(
    routes: SharedRoutes,
    port: u16,
    mut shutdown: ShutdownSignal,
) -> anyhow::Result<()> {
    let app = create_router(routes);

    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await?;

    info!("HTTP server listening on http://{}", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = shutdown.changed().await;
        })
        .await?;

    Ok(())
}

pub async fn run_https_server(
    routes: SharedRoutes,
    port: u16,
    tls_config: RustlsConfig,
    mut shutdown: ShutdownSignal,
) -> anyhow::Result<()> {
    let app = create_router(routes);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let handle = Handle::new();

    // Spawn task to handle shutdown
    let shutdown_handle = handle.clone();
    tokio::spawn(async move {
        let _ = shutdown.changed().await;
        shutdown_handle.graceful_shutdown(Some(Duration::from_secs(1)));
    });

    info!("HTTPS server listening on https://{}", addr);

    axum_server::bind_rustls(addr, tls_config)
        .handle(handle)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

async fn handler(State(state): State<Arc<AppState>>, request: Request<Body>) -> Response<Body> {
    let method = match *request.method() {
        Method::GET => HttpMethod::Get,
        Method::POST => HttpMethod::Post,
        Method::PUT => HttpMethod::Put,
        Method::DELETE => HttpMethod::Delete,
        Method::PATCH => HttpMethod::Patch,
        Method::HEAD => HttpMethod::Head,
        Method::OPTIONS => HttpMethod::Options,
        _ => {
            return Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .body(Body::from("Method not allowed"))
                .unwrap();
        }
    };

    let path = request.uri().path().to_string();

    // Find matching route using pattern matching
    let routes = state.routes.read().await;
    let route = routes
        .iter()
        .find(|route| route.method == method && route.matches(&path))
        .cloned();
    drop(routes);

    let route = match route {
        Some(r) => r,
        None => {
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from(format!(
                    "Route not found: {} {}",
                    request.method(),
                    path
                )))
                .unwrap();
        }
    };

    // Apply delay if configured
    if route.response.meta.delay > 0 {
        sleep(Duration::from_millis(route.response.meta.delay)).await;
    }

    // Build response
    let mut builder = Response::builder()
        .status(StatusCode::from_u16(route.response.meta.status).unwrap_or(StatusCode::OK));

    // Set content-type from file extension (can be overridden by headers)
    builder = builder.header("Content-Type", &route.content_type);

    // Apply custom headers
    for (name, value) in &route.response.meta.headers {
        if let (Ok(header_name), Ok(header_value)) = (
            HeaderName::try_from(name.as_str()),
            HeaderValue::try_from(value.as_str()),
        ) {
            builder = builder.header(header_name, header_value);
        }
    }

    builder
        .body(Body::from(route.response.body.clone()))
        .unwrap()
}
