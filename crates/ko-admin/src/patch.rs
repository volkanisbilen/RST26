//! HTTP half of the launcher patch system: serves the versioned zip files the real JstKO
//! Launcher (`NTT2625-d3d9proxy/Launcher`, the NLauncher-based project) downloads over HTTP
//! (its `FTP_PROTOCOL::HSACSX` is, despite the name, a plain HTTP GET via wininet). The TCP
//! half (`patch_tcp.rs`) tells the launcher which zip(s) to fetch; this just hands over the
//! bytes.
//!
//! `GET /patches/{file}` — raw bytes of one zip under `JSTKO_PATCH_DIR/zips/` (default
//! `/opt/ko26xx-build-2625/client_patch_release/zips`), built by `package_patch_zip.py`.
//! `file` must be `<digits>.zip` — nothing else is ever served from this route.

use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};

use crate::App;

pub fn routes() -> Router<App> {
    Router::new().route("/patches/{file}", get(file))
}

fn is_patch_zip_name(file: &str) -> bool {
    let Some(stem) = file.strip_suffix(".zip") else { return false };
    !stem.is_empty() && stem.chars().all(|c| c.is_ascii_digit())
}

async fn file(State(app): State<App>, Path(file): Path<String>) -> Response {
    if !is_patch_zip_name(&file) {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let full = app.patch_dir.join("zips").join(&file);
    match tokio::fs::read(&full).await {
        Ok(bytes) => (
            [(header::CONTENT_TYPE, "application/zip"), (header::CACHE_CONTROL, "no-store")],
            bytes,
        )
            .into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}
