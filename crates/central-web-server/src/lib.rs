//! baylee.app's web server (`docs/proposals/frontend-rewrite.md` §3): it
//! renders every page (`central-web`), serves the built assets, writes the
//! sitemap and reads the public data (GitHub, Central's directory). It has
//! no database, no secret and no session: account requests go from the
//! browser to Central's browser listener (`/api/*`, routed by Caddy), and
//! this process never sees the session cookie.
#![forbid(unsafe_code)]

pub mod config;
pub mod fetch;
pub mod files;
pub mod generated;
pub mod routes;

use std::sync::{Arc, RwLock};

use config::Config;
use files::{Broken, Files};
use routes::{App, Pages};

/// Builds the application from its settings: reads the assets and the
/// fallback data, but starts no task and opens no socket.
///
/// # Errors
/// When the build's assets cannot be read.
pub fn app(config: Config) -> Result<Arc<App>, Broken> {
    let (assets, files) = Files::load(&config.site)?;
    let data = Arc::new(RwLock::new(Arc::new(fetch::fallback(
        config.fallback.as_deref(),
    ))));
    Ok(Arc::new(App {
        config,
        assets,
        files,
        data,
        pages: Pages::default(),
    }))
}
