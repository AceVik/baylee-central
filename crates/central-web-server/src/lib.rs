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
pub mod views;

use std::sync::{Arc, RwLock};

use config::Config;
use files::{Broken, Files};
use routes::{App, Pages};

/// Builds the application from its settings: reads the assets and the
/// fallback data, but starts no task and opens no socket.
///
/// # Errors
/// When the build's assets cannot be read, or the file for the count of
/// page views cannot be used.
pub fn app(config: Config) -> Result<Arc<App>, Broken> {
    let (assets, files) = Files::load(&config.site)?;
    let mut data = fetch::fallback(config.fallback.as_deref());
    // the count the deployment keeps, from the first page on
    let views = match config.views.as_deref() {
        Some(file) => {
            let views = views::Views::open(file).map_err(|unusable| Broken(unusable.0.into()))?;
            data.views = Some(views.now());
            Some(Arc::new(views))
        }
        None => None,
    };
    Ok(Arc::new(App {
        config,
        assets,
        files,
        data: Arc::new(RwLock::new(Arc::new(data))),
        pages: Pages::default(),
        views,
    }))
}
