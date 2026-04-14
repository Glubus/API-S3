use crate::controllers::resources;
use axum::{
    Router,
    routing::{get, post},
};
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};

pub fn router() -> Router {
    Router::new()
        // GET pour récupérer une ressource unique avec query params (width, target)
        .route("/r/{category}/{*file_path}", get(resources::get_resource))
        // POST pour un batch de ressources
        .route("/r/bulk", post(resources::get_resources_bulk))
        // Documentation Scalar UI
        .merge(Scalar::with_url("/docs", resources::ApiDoc::openapi()))
}
