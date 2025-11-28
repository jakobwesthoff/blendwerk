/*
 * Copyright (c) 2025 Jakob Westhoff <jakob@westhoffswelt.de>
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::request_logger::{self, RequestLogger};
use crate::routes::{HttpMethod, Route};
use axum::{
    Router,
    body::Body,
    extract::State,
    http::{HeaderName, HeaderValue, Method, Request, StatusCode, request::Parts},
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
    pub request_logger: Option<RequestLogger>,
}

fn create_router(state: Arc<AppState>) -> Router {
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
    state: Arc<AppState>,
    port: u16,
    mut shutdown: ShutdownSignal,
) -> anyhow::Result<()> {
    let router = create_router(state);

    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await?;

    info!("HTTP server listening on http://{}", addr);

    axum::serve(listener, router)
        .with_graceful_shutdown(async move {
            let _ = shutdown.changed().await;
        })
        .await?;

    Ok(())
}

pub async fn run_https_server(
    state: Arc<AppState>,
    port: u16,
    tls_config: RustlsConfig,
    mut shutdown: ShutdownSignal,
) -> anyhow::Result<()> {
    let router = create_router(state);

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
        .serve(router.into_make_service())
        .await?;

    Ok(())
}

/// Response builder that encapsulates both HTTP response and logging info
struct ResponseBuilder {
    response: Response<Body>,
    info: request_logger::ResponseInfo,
    matched_route: Option<String>,
    request_info: Option<request_logger::RequestInfo>,
}

impl ResponseBuilder {
    fn method_not_allowed() -> Self {
        let body = "Method not allowed";
        Self {
            response: Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .body(Body::from(body))
                .unwrap(),
            info: request_logger::ResponseInfo {
                status: 405,
                headers: std::collections::HashMap::new(),
                body: body.to_string(),
                delay_ms: 0,
            },
            matched_route: None,
            request_info: None,
        }
    }

    fn not_found(method: &Method, path: &str) -> Self {
        let body = format!("Route not found: {} {}", method, path);
        Self {
            response: Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from(body.clone()))
                .unwrap(),
            info: request_logger::ResponseInfo {
                status: 404,
                headers: std::collections::HashMap::new(),
                body,
                delay_ms: 0,
            },
            matched_route: None,
            request_info: None,
        }
    }

    async fn from_route(route: Route) -> Self {
        // Apply delay if configured
        if route.response.meta.delay > 0 {
            sleep(Duration::from_millis(route.response.meta.delay)).await;
        }

        let matched_route = Some(route.display_path());

        // Build response
        let mut builder = Response::builder()
            .status(StatusCode::from_u16(route.response.meta.status).unwrap_or(StatusCode::OK));

        // Set content-type from file extension (can be overridden by headers)
        builder = builder.header("Content-Type", &route.content_type);

        // Collect headers for response info
        let mut response_headers = std::collections::HashMap::new();
        response_headers.insert("content-type".to_string(), route.content_type.clone());

        // Apply custom headers
        for (name, value) in &route.response.meta.headers {
            if let (Ok(header_name), Ok(header_value)) = (
                HeaderName::try_from(name.as_str()),
                HeaderValue::try_from(value.as_str()),
            ) {
                builder = builder.header(header_name, header_value);
                response_headers.insert(name.clone(), value.clone());
            }
        }

        let response_body = route.response.body.clone();
        let response = builder.body(Body::from(response_body.clone())).unwrap();

        Self {
            response,
            info: request_logger::ResponseInfo {
                status: route.response.meta.status,
                headers: response_headers,
                body: response_body,
                delay_ms: route.response.meta.delay,
            },
            matched_route,
            request_info: None,
        }
    }

    fn with_request_info(mut self, request_info: Option<request_logger::RequestInfo>) -> Self {
        self.request_info = request_info;
        self
    }

    fn log_and_return(self, state: &AppState) -> Response<Body> {
        // Log if enabled
        if let (Some(logger), Some(req_info)) = (&state.request_logger, self.request_info) {
            let logged =
                request_logger::create_logged_request(req_info, self.info, self.matched_route);
            logger.log_request_async(logged);
        }

        self.response
    }
}

/// Extract request information for logging if enabled
async fn extract_request_for_logging(
    state: &AppState,
    parts: &Parts,
    body: Body,
) -> Option<request_logger::RequestInfo> {
    if state.request_logger.is_none() {
        return None;
    }

    match request_logger::extract_request_info(&parts.method, &parts.uri, &parts.headers, body)
        .await
    {
        Ok(info) => Some(info),
        Err(e) => {
            tracing::error!("Failed to extract request info for logging: {}", e);
            None
        }
    }
}

/// Parse HTTP method to our internal enum
fn parse_http_method(method: &Method) -> Option<HttpMethod> {
    match *method {
        Method::GET => Some(HttpMethod::Get),
        Method::POST => Some(HttpMethod::Post),
        Method::PUT => Some(HttpMethod::Put),
        Method::DELETE => Some(HttpMethod::Delete),
        Method::PATCH => Some(HttpMethod::Patch),
        Method::HEAD => Some(HttpMethod::Head),
        Method::OPTIONS => Some(HttpMethod::Options),
        _ => None,
    }
}

/// Find a matching route for the request
async fn find_matching_route(state: &AppState, method: HttpMethod, path: &str) -> Option<Route> {
    let routes = state.routes.read().await;
    routes
        .iter()
        .find(|r| r.method == method && r.matches(path))
        .cloned()
}

async fn handler(State(state): State<Arc<AppState>>, request: Request<Body>) -> Response<Body> {
    let (parts, body) = request.into_parts();

    // Extract request information for logging
    let request_info = extract_request_for_logging(&state, &parts, body).await;

    // Parse HTTP method
    let method = match parse_http_method(&parts.method) {
        Some(m) => m,
        None => {
            return ResponseBuilder::method_not_allowed()
                .with_request_info(request_info)
                .log_and_return(&state);
        }
    };

    // Find matching route
    let path = parts.uri.path();
    let route = find_matching_route(&state, method, path).await;

    // Build and return response
    let response_builder = match route {
        Some(route) => ResponseBuilder::from_route(route).await,
        None => ResponseBuilder::not_found(&parts.method, path),
    };

    response_builder
        .with_request_info(request_info)
        .log_and_return(&state)
}
