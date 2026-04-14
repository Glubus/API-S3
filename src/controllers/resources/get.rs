use crate::models::{device::DeviceConfig, resources::ResourceCategory};
use crate::services::image_processor;
use axum::{
    body::Body,
    extract::{Path, Query},
    http::{StatusCode, header},
    response::IntoResponse,
};
use serde::Deserialize;
use std::str::FromStr;

#[derive(Debug, Deserialize)]
pub struct ResourceQuery {
    pub width: Option<u32>,
    pub target: Option<String>,
}

/// Fetches a single resource and processes it according to URL parameters.
///
/// # Route
/// `GET /r/{category}/{*file_path}?width=300&target=Phone`
#[utoipa::path(
    get,
    path = "/r/{category}/{*file_path}",
    params(
        ("category" = String, Path, description = "Resource category"),
        ("file_path" = String, Path, description = "Path to the resource within the category"),
        ("width" = Option<u32>, Query, description = "Width of the target image"),
        ("target" = Option<String>, Query, description = "Device target (phone, tablet, desktop)"),
    ),
    responses(
        (status = 200, description = "Resource found and processed", body = Vec<u8>),
        (status = 400, description = "Invalid parameters"),
        (status = 404, description = "Resource not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
#[tracing::instrument]
pub async fn get_resource(
    Path((category, file_path)): Path<(String, String)>,
    Query(query): Query<ResourceQuery>,
) -> impl IntoResponse {
    let width = query.width.unwrap_or(0);
    let target_str = query.target.unwrap_or_else(|| "desktop".to_string());

    if width == 0 || width > 4000 {
        tracing::warn!("Invalid width requested: {}", width);
        return (StatusCode::BAD_REQUEST, "Width must be between 1 and 4000").into_response();
    }

    let config = match crate::models::device::DeviceTarget::from_str(&target_str) {
        Ok(t) => DeviceConfig {
            target: t,
            width,
            height: 0, // Not strictly used or calculated dynamically? (Assuming standard behavior)
            scale: None,
        },
        Err(e) => return (StatusCode::BAD_REQUEST, e).into_response(),
    };

    let resource_path = file_path;
    // Validate category
    let category = match ResourceCategory::from_str(&category) {
        Ok(cat) => cat,
        Err(e) => {
            tracing::warn!("Invalid category: {}", e);
            return (StatusCode::BAD_REQUEST, format!("Invalid category: {e}")).into_response();
        }
    };

    // Guard against path traversal
    if resource_path.contains("..") {
        tracing::warn!("Invalid path traversal attempt: {}", resource_path);
        return (StatusCode::BAD_REQUEST, "Invalid path").into_response();
    }

    // Build S3 key
    let s3_key = format!("{}/{}", category.s3_prefix(), resource_path);

    // Fetch and process image
    match image_processor::get_or_process_cached(s3_key, config).await {
        Ok(processed_bytes) => (
            [(header::CONTENT_TYPE, "image/jpeg")],
            Body::from(processed_bytes),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Resource retrieval/processing failed: {}", e);
            (e.status_code(), e.to_string()).into_response()
        }
    }
}
