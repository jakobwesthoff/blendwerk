/*
 * Copyright (c) 2025 Jakob Westhoff <jakob@westhoffswelt.de>
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use anyhow::{Context, Result};
use axum::body::Body;
use axum::http::{HeaderMap, Method, Uri};
use clap::ValueEnum;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use tracing::error;

#[derive(Debug, Clone, ValueEnum)]
pub enum LogFormat {
    Json,
    Yaml,
}

impl LogFormat {
    fn extension(&self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Yaml => "yaml",
        }
    }

    fn serialize<T: Serialize>(&self, value: &T) -> Result<Vec<u8>> {
        match self {
            Self::Json => serde_json::to_vec_pretty(value).context("Failed to serialize to JSON"),
            Self::Yaml => {
                let yaml_string =
                    serde_yaml::to_string(value).context("Failed to serialize to YAML")?;
                Ok(yaml_string.into_bytes())
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct RequestLogger {
    base_dir: PathBuf,
    format: LogFormat,
}

impl RequestLogger {
    pub fn new(base_dir: PathBuf, format: LogFormat) -> Self {
        Self { base_dir, format }
    }

    /// Log a request asynchronously. This method spawns a task and never blocks.
    pub fn log_request_async(&self, logged_request: LoggedRequest) {
        let logger = self.clone();
        tokio::spawn(async move {
            if let Err(e) = logger.log_request(logged_request).await {
                error!("Failed to log request: {}", e);
            }
        });
    }

    async fn log_request(&self, logged_request: LoggedRequest) -> Result<()> {
        // Build directory path: base_dir/path/METHOD/
        let request_path = logged_request
            .request
            .path
            .trim_start_matches('/')
            .to_string();

        let method_str = logged_request.request.method.clone();

        let dir_path = if request_path.is_empty() {
            // Root path
            self.base_dir.join(method_str)
        } else {
            self.base_dir.join(&request_path).join(method_str)
        };

        // Create directory structure
        fs::create_dir_all(&dir_path)
            .await
            .context("Failed to create log directory")?;

        // Generate filename: timestamp_ulid.extension
        let filename = format!(
            "{}_{}.{}",
            logged_request.metadata.timestamp,
            logged_request.metadata.request_id,
            self.format.extension()
        );

        let file_path = dir_path.join(filename);

        // Serialize and write
        let content = self.format.serialize(&logged_request)?;
        fs::write(&file_path, content)
            .await
            .context("Failed to write log file")?;

        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct LoggedRequest {
    pub metadata: RequestMetadata,
    pub request: RequestInfo,
    pub response: ResponseInfo,
}

#[derive(Debug, Serialize)]
pub struct RequestMetadata {
    pub timestamp: String,
    pub request_id: String,
}

#[derive(Debug, Serialize)]
pub struct RequestInfo {
    pub method: String,
    pub uri: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    pub headers: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_route: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ResponseInfo {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub delay_ms: u64,
}

/// Extract request information for logging
pub async fn extract_request_info(
    method: &Method,
    uri: &Uri,
    headers: &HeaderMap,
    body: Body,
) -> Result<RequestInfo> {
    // Read body
    let body_bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .context("Failed to read request body")?;

    let body_string = if body_bytes.is_empty() {
        None
    } else {
        Some(String::from_utf8_lossy(&body_bytes).to_string())
    };

    // Convert headers to HashMap
    let headers_map: HashMap<String, String> = headers
        .iter()
        .map(|(name, value)| {
            (
                name.to_string(),
                value.to_str().unwrap_or("<binary>").to_string(),
            )
        })
        .collect();

    let request_info = RequestInfo {
        method: method.to_string(),
        uri: uri.to_string(),
        path: uri.path().to_string(),
        query: uri.query().map(String::from),
        headers: headers_map,
        body: body_string,
        matched_route: None, // Will be set later if route is found
    };

    Ok(request_info)
}

/// Create a complete LoggedRequest from all components
pub fn create_logged_request(
    mut request_info: RequestInfo,
    response_info: ResponseInfo,
    matched_route: Option<String>,
) -> LoggedRequest {
    // Set the matched route
    request_info.matched_route = matched_route;

    // Generate metadata
    let now = chrono::Utc::now();
    let timestamp = now.format("%Y-%m-%dT%H-%M-%S%.6fZ").to_string();
    let request_id = ulid::Ulid::new().to_string();

    LoggedRequest {
        metadata: RequestMetadata {
            timestamp,
            request_id,
        },
        request: request_info,
        response: response_info,
    }
}
