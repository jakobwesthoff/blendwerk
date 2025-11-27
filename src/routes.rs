/*
 * Copyright (c) 2025 Jakob Westhoff <jakob@westhoffswelt.de>
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::frontmatter::{ParsedResponse, parse_frontmatter};
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
}

impl HttpMethod {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "get" => Some(Self::Get),
            "post" => Some(Self::Post),
            "put" => Some(Self::Put),
            "delete" => Some(Self::Delete),
            "patch" => Some(Self::Patch),
            "head" => Some(Self::Head),
            "options" => Some(Self::Options),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Route {
    pub method: HttpMethod,
    pub path_segments: Vec<PathSegment>,
    pub response: ParsedResponse,
    pub content_type: String,
}

#[derive(Debug, Clone)]
pub enum PathSegment {
    Static(String),
    Dynamic(String), // Parameter name
}

impl Route {
    /// Generate a display path for logging (e.g., "/users/:id")
    pub fn display_path(&self) -> String {
        if self.path_segments.is_empty() {
            return "/".to_string();
        }

        let parts: Vec<String> = self
            .path_segments
            .iter()
            .map(|segment| match segment {
                PathSegment::Static(s) => s.clone(),
                PathSegment::Dynamic(name) => format!(":{}", name),
            })
            .collect();

        format!("/{}", parts.join("/"))
    }

    pub fn matches(&self, request_path: &str) -> bool {
        let request_segments: Vec<&str> = request_path
            .trim_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();

        let pattern_len = self.path_segments.len();

        if request_segments.len() != pattern_len {
            // Handle root path special case
            if pattern_len == 0 && request_segments.is_empty() {
                return true;
            }
            return false;
        }

        for (segment, pattern) in request_segments.iter().zip(&self.path_segments) {
            match pattern {
                PathSegment::Static(s) => {
                    if s != segment {
                        return false;
                    }
                }
                PathSegment::Dynamic(_) => {
                    // Dynamic segments match anything
                }
            }
        }

        true
    }
}

pub fn scan_directory(base_dir: &Path) -> Result<Vec<Route>> {
    let mut routes = Vec::new();
    scan_dir_recursive(base_dir, base_dir, &mut routes)?;
    Ok(routes)
}

fn scan_dir_recursive(base_dir: &Path, current_dir: &Path, routes: &mut Vec<Route>) -> Result<()> {
    let entries = fs::read_dir(current_dir)
        .with_context(|| format!("Failed to read directory: {}", current_dir.display()))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            scan_dir_recursive(base_dir, &path, routes)?;
        } else if path.is_file()
            && let Some(route) = parse_route_file(base_dir, &path)?
        {
            routes.push(route);
        }
    }

    Ok(())
}

fn parse_route_file(base_dir: &Path, file_path: &Path) -> Result<Option<Route>> {
    let file_name = file_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

    let extension = file_path.extension().and_then(|s| s.to_str()).unwrap_or("");

    // Parse HTTP method from filename (case-insensitive)
    let method = match HttpMethod::from_str(file_name) {
        Some(m) => m,
        None => return Ok(None), // Not a valid route file
    };

    // Build path segments from directory structure
    let parent = file_path.parent().unwrap_or(base_dir);
    let relative_path = parent.strip_prefix(base_dir).unwrap_or(Path::new(""));

    // Parse path segments and identify dynamic parameters
    let mut path_segments = Vec::new();

    for component in relative_path.components() {
        if let std::path::Component::Normal(os_str) = component {
            let segment = os_str.to_string_lossy();
            if segment.starts_with('[') && segment.ends_with(']') {
                // Dynamic parameter: [id]
                let param_name = &segment[1..segment.len() - 1];
                path_segments.push(PathSegment::Dynamic(param_name.to_string()));
            } else {
                path_segments.push(PathSegment::Static(segment.to_string()));
            }
        }
    }

    // Determine content type from extension
    let content_type = match extension {
        "json" => "application/json",
        "html" | "htm" => "text/html",
        "xml" => "application/xml",
        "txt" => "text/plain",
        "css" => "text/css",
        "js" => "application/javascript",
        _ => "application/octet-stream",
    }
    .to_string();

    // Read and parse file content
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

    let response = parse_frontmatter(&content)
        .with_context(|| format!("Failed to parse frontmatter in: {}", file_path.display()))?;

    Ok(Some(Route {
        method,
        path_segments,
        response,
        content_type,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_scan_simple_directory() {
        let temp_dir = TempDir::new().unwrap();
        let api_dir = temp_dir.path().join("api");
        fs::create_dir(&api_dir).unwrap();

        // Create GET.json
        fs::write(
            api_dir.join("GET.json"),
            r#"---
status: 200
---
{"message": "hello"}"#,
        )
        .unwrap();

        // Create post.json (lowercase)
        fs::write(api_dir.join("post.json"), r#"{"created": true}"#).unwrap();

        let routes = scan_directory(temp_dir.path()).unwrap();

        assert_eq!(routes.len(), 2);
        assert!(
            routes
                .iter()
                .any(|r| r.method == HttpMethod::Get && r.display_path() == "/api")
        );
        assert!(
            routes
                .iter()
                .any(|r| r.method == HttpMethod::Post && r.display_path() == "/api")
        );
    }

    #[test]
    fn test_content_type_inference() {
        let temp_dir = TempDir::new().unwrap();

        fs::write(temp_dir.path().join("GET.json"), "{}").unwrap();
        fs::write(temp_dir.path().join("POST.html"), "<html></html>").unwrap();
        fs::write(temp_dir.path().join("PUT.txt"), "text").unwrap();

        let routes = scan_directory(temp_dir.path()).unwrap();

        let json_route = routes
            .iter()
            .find(|r| r.method == HttpMethod::Get && r.display_path() == "/")
            .unwrap();
        assert_eq!(json_route.content_type, "application/json");

        let html_route = routes
            .iter()
            .find(|r| r.method == HttpMethod::Post && r.display_path() == "/")
            .unwrap();
        assert_eq!(html_route.content_type, "text/html");

        let txt_route = routes
            .iter()
            .find(|r| r.method == HttpMethod::Put && r.display_path() == "/")
            .unwrap();
        assert_eq!(txt_route.content_type, "text/plain");
    }

    #[test]
    fn test_path_parameters() {
        let temp_dir = TempDir::new().unwrap();
        let users_dir = temp_dir.path().join("users").join("[id]");
        fs::create_dir_all(&users_dir).unwrap();

        fs::write(users_dir.join("GET.json"), r#"{"user": "test"}"#).unwrap();

        let routes = scan_directory(temp_dir.path()).unwrap();

        assert_eq!(routes.len(), 1);

        // Check the path uses :id syntax
        let route = routes
            .iter()
            .find(|r| r.method == HttpMethod::Get && r.display_path() == "/users/:id")
            .unwrap();
        assert_eq!(route.display_path(), "/users/:id");

        // Test pattern matching
        assert!(route.matches("/users/123"));
        assert!(route.matches("/users/abc"));
        assert!(!route.matches("/users"));
        assert!(!route.matches("/users/123/extra"));
    }
}
