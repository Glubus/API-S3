use api_s3::models::device::{DeviceConfig, DeviceTarget};
use api_s3::routes;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::util::ServiceExt;

#[tokio::test]
async fn test_post_get_image() {
    let app = routes::resources::router();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/r/images/00584f147d171b9c512d37f7b9f696c3.png?width=300&target=Phone")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_post_get_image_not_found() {
    let app = routes::resources::router();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/r/images/non_existent.jpg?width=1920&target=Desktop")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_post_get_image_traversal() {
    let app = routes::resources::router();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/r/images/%2E%2E%2Fsecret.txt?width=1920&target=Desktop")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

use api_s3::models::resources::{BatchResourceItem, BatchResourceRequest};

#[tokio::test]
async fn test_batch_images() {
    let app = routes::resources::router();

    let request = BatchResourceRequest {
        config: DeviceConfig {
            target: DeviceTarget::Phone,
            width: 100,
            height: 100,
            scale: None,
        },
        items: vec![BatchResourceItem {
            category: "images".to_string(),
            path: "00584f147d171b9c512d37f7b9f696c3.png".to_string(),
        }],
    };

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/r/bulk")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(content_type.contains("multipart/mixed"));
}

#[tokio::test]
async fn test_get_image_invalid_width() {
    let app = routes::resources::router();

    // Width 0
    let mut response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/r/images/00584f147d171b9c512d37f7b9f696c3.png?width=0&target=Phone")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Width > 4000
    response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/r/images/00584f147d171b9c512d37f7b9f696c3.png?width=4001&target=Phone")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_batch_images_invalid_width() {
    let app = routes::resources::router();

    let request = BatchResourceRequest {
        config: DeviceConfig {
            target: DeviceTarget::Phone,
            width: 5000,
            height: 100,
            scale: None,
        },
        items: vec![],
    };

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/r/bulk")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ============================================================================
// MinIO Storage Service Tests
// ============================================================================

use api_s3::services::storage::StorageService;

#[tokio::test]
async fn test_minio_env_vars_loaded() {
    // Initialize environment from .env
    dotenvy::dotenv().ok();

    // Verify that environment variables are loaded
    let endpoint = std::env::var("S3_ENDPOINT").unwrap_or_else(|_| "NOT SET".to_string());
    let bucket = std::env::var("S3_BUCKET").unwrap_or_else(|_| "NOT SET".to_string());
    let access_key = std::env::var("S3_ACCESS_KEY").unwrap_or_else(|_| "NOT SET".to_string());
    let secret_key = std::env::var("S3_SECRET_KEY").unwrap_or_else(|_| "NOT SET".to_string());

    println!("S3_ENDPOINT: {}", endpoint);
    println!("S3_BUCKET: {}", bucket);
    println!("S3_ACCESS_KEY: {}", access_key);
    println!("S3_SECRET_KEY: {}", secret_key);

    assert_ne!(endpoint, "NOT SET", "S3_ENDPOINT should be set in .env");
    assert_ne!(bucket, "NOT SET", "S3_BUCKET should be set in .env");
    assert_ne!(access_key, "NOT SET", "S3_ACCESS_KEY should be set in .env");
    assert_ne!(secret_key, "NOT SET", "S3_SECRET_KEY should be set in .env");
}

#[tokio::test]
async fn test_s3_config_debug() {
    dotenvy::dotenv().ok();

    let endpoint = std::env::var("S3_ENDPOINT").unwrap_or_else(|_| "NOT SET".to_string());
    let bucket = std::env::var("S3_BUCKET").unwrap_or_else(|_| "NOT SET".to_string());
    let access_key = std::env::var("S3_ACCESS_KEY").unwrap_or_else(|_| "NOT SET".to_string());

    println!("\n=== S3 Configuration ===");
    println!("S3_ENDPOINT: {}", endpoint);
    println!("S3_BUCKET: {}", bucket);
    println!(
        "S3_ACCESS_KEY: {} (len: {})",
        if access_key.len() > 5 {
            format!("{}***", &access_key[..5])
        } else {
            "VERY_SHORT".to_string()
        },
        access_key.len()
    );
    println!("========================\n");
}

#[tokio::test]
async fn test_minio_retrieve_icon_ampoule() {
    // Initialize environment from .env
    dotenvy::dotenv().ok();

    // Test retrieving icon_ampoule.png from MinIO
    let result = StorageService::get_image("icons/icon_ampoule.png").await;

    match result {
        Ok(bytes) => {
            assert!(!bytes.is_empty(), "icon_ampoule.png should not be empty");
            println!(
                "✓ Successfully retrieved icon_ampoule.png from MinIO ({} bytes)",
                bytes.len()
            );
        }
        Err(e) => {
            println!("Error details: {:?}", e);
            panic!("Failed to retrieve icon_ampoule.png from MinIO: {}", e);
        }
    }
}

#[tokio::test]
async fn test_minio_retrieve_icon_ampoule_is_png() {
    dotenvy::dotenv().ok();

    let result = StorageService::get_image("icons/icon_ampoule.png").await;

    assert!(
        result.is_ok(),
        "Should successfully retrieve icon_ampoule.png"
    );

    let bytes = result.unwrap();

    // Check PNG magic bytes: 0x89 0x50 0x4E 0x47 (PNG signature)
    assert!(bytes.len() >= 8, "PNG file should be at least 8 bytes long");
    assert_eq!(
        &bytes[0..4],
        &[0x89, 0x50, 0x4E, 0x47],
        "File should start with PNG magic bytes"
    );

    println!("✓ icon_ampoule.png is a valid PNG file");
}

#[tokio::test]
async fn test_minio_retrieve_nonexistent_file() {
    dotenvy::dotenv().ok();

    let result = StorageService::get_image("nonexistent_icon.png").await;

    assert!(
        result.is_err(),
        "Should fail when retrieving nonexistent file"
    );

    let error = result.unwrap_err();
    let error_msg = error.to_string();
    println!("Error for nonexistent file: {:?}", error);
    println!("Error message: {}", error_msg);
    assert!(
        error_msg.contains("not found") || error_msg.contains("404"),
        "Error should indicate file not found, got: {}",
        error_msg
    );

    println!("✓ Correctly handles nonexistent file retrieval");
}

#[tokio::test]
async fn test_minio_path_traversal_protection() {
    dotenvy::dotenv().ok();

    // Attempt directory traversal attack
    let result = StorageService::get_image("icons/../../../etc/passwd").await;

    assert!(result.is_err(), "Should reject path traversal attempts");

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("Invalid key"),
        "Should indicate invalid key"
    );

    println!("✓ Path traversal protection works correctly");
}
