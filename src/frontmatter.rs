/*
 * Copyright (c) 2025 Jakob Westhoff <jakob@westhoffswelt.de>
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct ResponseMeta {
    #[serde(default = "default_status")]
    pub status: u16,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub delay: u64,
}

fn default_status() -> u16 {
    200
}

impl Default for ResponseMeta {
    fn default() -> Self {
        Self {
            status: 200,
            headers: HashMap::new(),
            delay: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParsedResponse {
    pub meta: ResponseMeta,
    pub body: String,
}

pub fn parse_frontmatter(content: &str) -> Result<ParsedResponse> {
    let content = content.trim_start();

    // Check if content starts with frontmatter delimiter
    if !content.starts_with("---") {
        // No frontmatter, entire content is body
        return Ok(ParsedResponse {
            meta: ResponseMeta::default(),
            body: content.to_string(),
        });
    }

    // Find the closing delimiter
    let after_first = &content[3..];
    let closing_pos = after_first
        .find("\n---")
        .ok_or_else(|| anyhow::anyhow!("Missing closing frontmatter delimiter '---'"))?;

    let yaml_content = &after_first[..closing_pos].trim();
    let body_start = 3 + closing_pos + 4; // Skip "---" + yaml + "\n---"

    let body = if body_start < content.len() {
        content[body_start..].trim_start_matches('\n').to_string()
    } else {
        String::new()
    };

    let meta: ResponseMeta = if yaml_content.is_empty() {
        ResponseMeta::default()
    } else {
        serde_yaml::from_str(yaml_content).context("Failed to parse YAML frontmatter")?
    };

    Ok(ParsedResponse { meta, body })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_frontmatter() {
        let content = r#"{"hello": "world"}"#;
        let result = parse_frontmatter(content).unwrap();
        assert_eq!(result.meta.status, 200);
        assert_eq!(result.body, r#"{"hello": "world"}"#);
    }

    #[test]
    fn test_with_frontmatter() {
        let content = r#"---
status: 201
headers:
  X-Custom: value
delay: 100
---
{"created": true}"#;
        let result = parse_frontmatter(content).unwrap();
        assert_eq!(result.meta.status, 201);
        assert_eq!(result.meta.delay, 100);
        assert_eq!(result.meta.headers.get("X-Custom").unwrap(), "value");
        assert_eq!(result.body, r#"{"created": true}"#);
    }

    #[test]
    fn test_empty_frontmatter() {
        let content = r#"---
---
body content"#;
        let result = parse_frontmatter(content).unwrap();
        assert_eq!(result.meta.status, 200);
        assert_eq!(result.body, "body content");
    }

    #[test]
    fn test_partial_frontmatter() {
        let content = r#"---
status: 404
---
Not found"#;
        let result = parse_frontmatter(content).unwrap();
        assert_eq!(result.meta.status, 404);
        assert!(result.meta.headers.is_empty());
        assert_eq!(result.body, "Not found");
    }
}
