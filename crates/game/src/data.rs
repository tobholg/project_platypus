//! Data files under `assets/data/` and hot reload. Gameplay tuning lives in
//! RON, and editing a file while the game runs applies it (SPEC §7).

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use serde::de::DeserializeOwned;

pub fn assets_dir() -> PathBuf {
    std::env::var("BEVY_ASSET_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../..")))
        .join("assets")
}

pub fn data_path(rel: &str) -> PathBuf {
    assets_dir().join("data").join(rel)
}

/// RON with the extensions content authors expect (`Some` may be omitted).
pub fn parse_ron<T: DeserializeOwned>(text: &str) -> Result<T, String> {
    ron::Options::default()
        .with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME)
        .from_str(text)
        .map_err(|e| e.to_string())
}

pub fn load_ron<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse_ron(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Polls a file or directory for changes (cheap; twice a second).
pub struct Watched {
    path: PathBuf,
    stamp: Option<SystemTime>,
    last_check: Instant,
}

impl Watched {
    pub fn new(path: PathBuf) -> Self {
        let stamp = newest_mtime(&path);
        Watched { path, stamp, last_check: Instant::now() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// True once after the file (or any file in the directory) changed.
    pub fn changed(&mut self) -> bool {
        if self.last_check.elapsed() < Duration::from_millis(500) {
            return false;
        }
        self.last_check = Instant::now();
        let stamp = newest_mtime(&self.path);
        if stamp == self.stamp {
            return false;
        }
        self.stamp = stamp;
        true
    }
}

fn newest_mtime(path: &Path) -> Option<SystemTime> {
    let meta = std::fs::metadata(path).ok()?;
    if meta.is_dir() {
        std::fs::read_dir(path).ok()?.filter_map(|e| e.ok()?.metadata().ok()?.modified().ok()).max()
    } else {
        meta.modified().ok()
    }
}
