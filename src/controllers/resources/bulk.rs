use crate::models::device::DeviceConfig;
use crate::models::resources::{BatchResourceItem, BatchResourceRequest, ResourceCategory};
use axum::{
    Json,
    body::{Body, Bytes},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use std::str::FromStr;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio_stream::wrappers::ReceiverStream;

fn validate_request(payload: &BatchResourceRequest) -> Result<(), Box<Response>> {
    if payload.items.len() > 50 {
        return Err(Box::new(
            (StatusCode::BAD_REQUEST, "Batch limited to 50 items").into_response(),
        ));
    }
    if payload.config.width == 0 || payload.config.width > 4000 {
        tracing::warn!("Invalid width requested in batch: {}", payload.config.width);
        return Err(Box::new(
            (StatusCode::BAD_REQUEST, "Width must be between 1 and 4000").into_response(),
        ));
    }
    if payload.items.is_empty() {
        return Err(Box::new(StatusCode::NOT_FOUND.into_response()));
    }
    Ok(())
}

async fn process_item(
    item: BatchResourceItem,
    config: DeviceConfig,
) -> Result<(String, Vec<u8>), String> {
    let Ok(category) = ResourceCategory::from_str(&item.category) else {
        return Err(format!("Invalid category: {}", item.category));
    };
    if item.path.contains("..") {
        return Err("Invalid path".to_string());
    }
    let s3_key = format!("{}/{}", category.s3_prefix(), item.path);
    match crate::services::image_processor::get_or_process_cached(s3_key, config).await {
        Ok(data) => Ok((item.path, data)),
        Err(e) => Err(e.to_string()),
    }
}

fn build_multipart_chunk(name: &str, data: &[u8], boundary: &str) -> Bytes {
    let mut chunk = Vec::new();
    chunk.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    chunk.extend_from_slice(b"Content-Type: image/jpeg\r\n");
    chunk.extend_from_slice(
        format!("Content-Disposition: attachment; filename=\"{name}.jpg\"\r\n\r\n").as_bytes(),
    );
    chunk.extend_from_slice(data);
    chunk.extend_from_slice(b"\r\n");
    Bytes::from(chunk)
}

fn spawn_stream(
    mut set: JoinSet<Result<(String, Vec<u8>), String>>,
    tx: mpsc::Sender<Result<Bytes, std::convert::Infallible>>,
    boundary: &'static str,
) {
    tokio::spawn(async move {
        let mut has_items = false;

        while let Some(res) = set.join_next().await {
            match res {
                Ok(Ok((name, data))) => {
                    has_items = true;
                    let chunk = build_multipart_chunk(&name, &data, boundary);
                    if tx.send(Ok(chunk)).await.is_err() {
                        tracing::warn!("Client disconnected before stream finished");
                        break;
                    }
                }
                Ok(Err(e)) => {
                    tracing::warn!("Batch item skipped: {}", e);
                }
                Err(e) => {
                    tracing::error!("Join error: {}", e);
                }
            }
        }

        if has_items {
            let final_boundary = format!("--{boundary}--\r\n");
            let _ = tx.send(Ok(Bytes::from(final_boundary))).await;
        }
    });
}

/// Récupère un batch de ressources en parallèle
///
/// # Route
/// `POST /r/bulk`
///
/// # Body
/// ```json
/// {
///   "config": {
///     "target": "Phone",
///     "width": 300,
///     "height": 500,
///     "scale": null
///   },
///   "items": [
///     {"category": "icons", "path": "musee-001/avatar.png"},
///     {"category": "images", "path": "photo.png"}
///   ]
/// }
/// ```
///
/// # Response
/// Multipart/mixed contenant toutes les ressources traitées
#[utoipa::path(
    post,
    path = "/r/bulk",
    request_body = BatchResourceRequest,
    responses(
        (status = 200, description = "Batch processed successfully", body = Vec<u8>),
        (status = 400, description = "Invalid request or batch exceeds 50 items"),
        (status = 404, description = "No resources found"),
        (status = 500, description = "Internal server error"),
    ),
)]
#[tracing::instrument(skip(payload))]
pub async fn get_resources_bulk(Json(payload): Json<BatchResourceRequest>) -> impl IntoResponse {
    if let Err(response) = validate_request(&payload) {
        return *response;
    }

    let config = payload.config;
    let items = payload.items;
    let mut set = JoinSet::new();

    for item in items {
        let config = config.clone();
        set.spawn(process_item(item, config));
    }

    let (tx, rx) = mpsc::channel::<Result<Bytes, std::convert::Infallible>>(10);
    spawn_stream(set, tx, "resource_batch_boundary");
    let stream = ReceiverStream::new(rx);

    (
        [(
            header::CONTENT_TYPE,
            "multipart/mixed; boundary=resource_batch_boundary",
        )],
        Body::from_stream(stream),
    )
        .into_response()
}
