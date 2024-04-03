use axum::{extract::{Request, State}, response::IntoResponse};

use crate::rm::Filesystem;



pub async fn handle_webdav(req: Request, State(fs): State<Filesystem>) -> impl IntoResponse {
    tracing::info!("{req:?}");
}