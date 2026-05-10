//! GET /api/health.

use axum::response::{IntoResponse, Response};
use axum::Json;

pub async fn api_health() -> Response {
    Json(serde_json::json!({"status": "ok"})).into_response()
}
