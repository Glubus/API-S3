use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

use api_s3::{routes, telemetry};
use axum::Router;
use opentelemetry::trace::TracerProvider as _;
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    // Initialise le TracerProvider OTLP → Jaeger
    let provider = telemetry::init_tracer();
    let tracer = provider.tracer("api_s3");

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .with(OpenTelemetryLayer::new(tracer))
        .init();

    let app = Router::new()
        .merge(routes::resources::router())
        // Crée un span tracing pour chaque requête HTTP entrante
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    // Flush tous les spans en attente avant de quitter
    telemetry::shutdown_tracer(&provider);
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for ctrl-c signal");
    tracing::info!("Shutdown signal received");
}
