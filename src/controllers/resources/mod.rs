#![allow(clippy::needless_for_each)]

pub mod bulk;
pub mod get;

pub use bulk::get_resources_bulk;
pub use get::get_resource;

use crate::models::device::DeviceConfig;
use crate::models::resources::{BatchResourceItem, BatchResourceRequest};

/// `OpenAPI` documentation for resources API
#[derive(utoipa::OpenApi)]
#[openapi(
    paths(get::get_resource, bulk::get_resources_bulk),
    components(schemas(DeviceConfig, BatchResourceRequest, BatchResourceItem)),
    info(
        title = "S3 Resources API",
        description = "API for retrieving and processing resources from S3 buckets",
        version = "0.1.0",
    )
)]
pub struct ApiDoc;
