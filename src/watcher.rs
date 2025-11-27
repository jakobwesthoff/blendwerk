/*
 * Copyright (c) 2025 Jakob Westhoff <jakob@westhoffswelt.de>
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::routes::scan_directory;
use crate::server::{SharedRoutes, ShutdownSignal};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::{error, info};

pub async fn watch_directory(
    dir: PathBuf,
    routes: SharedRoutes,
    mut shutdown: ShutdownSignal,
) -> anyhow::Result<()> {
    let (tx, mut rx) = mpsc::channel(100);

    let mut watcher = RecommendedWatcher::new(
        move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                if event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove() {
                    let _ = tx.blocking_send(());
                }
            }
        },
        notify::Config::default(),
    )?;

    watcher.watch(&dir, RecursiveMode::Recursive)?;

    info!("  Watching {} for changes", dir.display());

    // Keep watcher alive and process events
    loop {
        tokio::select! {
            Some(()) = rx.recv() => {
                // Debounce: wait a bit for multiple rapid changes
                sleep(Duration::from_millis(100)).await;

                // Drain any additional events
                while rx.try_recv().is_ok() {}

                // Rebuild routes
                match scan_directory(&dir) {
                    Ok(new_routes) => {
                        let count = new_routes.len();
                        let mut routes_guard = routes.write().await;
                        *routes_guard = new_routes;
                        drop(routes_guard);
                        info!("  Reloaded {} routes", count);
                    }
                    Err(e) => {
                        error!("  Error reloading routes: {}", e);
                    }
                }
            }
            _ = shutdown.changed() => {
                break;
            }
        }
    }

    Ok(())
}
