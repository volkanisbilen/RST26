//! Launcher patch endpoints: serves the packaged client patch set (`package_patch.py`,
//! run on the dev machine, uploaded to the VPS under `JSTKO_PATCH_DIR`, default
//! `/opt/ko26xx-build-2625/client_patch_release`).
//!
//! - `GET /jstko_patch/manifest.json` — `[{"path": "...", "sha256": "...", "size": N}, ...]`,
//!   one entry per file the launcher should have. Always read fresh off disk (no cache), so a
//!   new `package_patch.py` run takes effect without restarting ko-admin.
//! - `GET /jstko_patch/files/{*path}` — raw bytes of one file, `path` exactly as the manifest
//!   gives it (forward slashes). `..`, a leading slash, or a drive letter are refused; only
//!   file extensions the launcher is allowed to replace are served.

use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};

use crate::App;

pub fn routes() -> Router<App> {
    Router::new()
        .route("/jstko_patch/manifest.json", get(manifest))
        .route("/jstko_patch/files/{*path}", get(file))
}

async fn manifest(State(app): State<App>) -> Response {
    match tokio::fs::read(app.patch_dir.join("manifest.json")).await {
        Ok(bytes) => (
            [(header::CONTENT_TYPE, "application/json"), (header::CACHE_CONTROL, "no-store")],
            bytes,
        )
            .into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Extensions the launcher is ever allowed to fetch — keeps this endpoint from turning into an
/// arbitrary-file server even if `package_patch.py` or the manifest is ever wrong.
const ALLOWED_EXT: &[&str] = &["dll", "exe", "tbl", "pus", "n3chr", "hdr", "src"];

fn safe_relative_path(path: &str) -> Option<&str> {
    if path.is_empty() || path.contains("..") || path.contains(':') || path.starts_with('/') || path.starts_with('\\') {
        return None;
    }
    let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    if !ALLOWED_EXT.contains(&ext.as_str()) {
        return None;
    }
    Some(path)
}

async fn file(State(app): State<App>, Path(path): Path<String>) -> Response {
    let Some(rel) = safe_relative_path(&path) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    // The manifest uses forward slashes; the files/ tree on disk mirrors it the same way.
    let full = app.patch_dir.join("files").join(rel);
    match tokio::fs::read(&full).await {
        Ok(bytes) => (
            [(header::CONTENT_TYPE, "application/octet-stream"), (header::CACHE_CONTROL, "no-store")],
            bytes,
        )
            .into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}
