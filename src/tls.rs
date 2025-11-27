/*
 * Copyright (c) 2025 Jakob Westhoff <jakob@westhoffswelt.de>
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use anyhow::{Context, Result};
use axum_server::tls_rustls::RustlsConfig;
use rcgen::{CertifiedKey, generate_simple_self_signed};
use std::path::Path;

pub async fn create_self_signed_config() -> Result<RustlsConfig> {
    let subject_alt_names = vec![
        "localhost".to_string(),
        "127.0.0.1".to_string(),
        "::1".to_string(),
    ];

    let CertifiedKey { cert, signing_key } = generate_simple_self_signed(subject_alt_names)
        .context("Failed to generate self-signed certificate")?;

    let cert_pem = cert.pem();
    let key_pem = signing_key.serialize_pem();

    RustlsConfig::from_pem(cert_pem.into_bytes(), key_pem.into_bytes())
        .await
        .context("Failed to create TLS config from self-signed certificate")
}

pub async fn load_custom_config(cert_file: &Path, key_file: &Path) -> Result<RustlsConfig> {
    RustlsConfig::from_pem_file(cert_file, key_file)
        .await
        .with_context(|| {
            format!(
                "Failed to load TLS config from cert={} key={}",
                cert_file.display(),
                key_file.display()
            )
        })
}
