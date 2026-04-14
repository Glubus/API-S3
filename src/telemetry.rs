use opentelemetry::{KeyValue, global};
use opentelemetry_otlp::{SpanExporter, WithExportConfig};
use opentelemetry_sdk::{Resource, trace::SdkTracerProvider};
use opentelemetry_semantic_conventions::resource::{SERVICE_NAME, SERVICE_VERSION};

/// Initializes the OpenTelemetry pipeline → OTLP → Jaeger.
///
/// Environment variables:
/// - `OTEL_EXPORTER_OTLP_ENDPOINT`: gRPC OTLP endpoint
///   (default: `http://your-super-jaeger.svc.cluster.local:4317`)
/// - `OTEL_SERVICE_NAME`: service name reported in traces (default: crate name)
///
/// Returns the `SdkTracerProvider` for graceful shutdown.
///
/// # Panics
/// Panics if the OTLP exporter cannot be built (invalid endpoint, tonic unavailable).
#[must_use]
pub fn init_tracer() -> SdkTracerProvider {
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").unwrap_or_else(|_| {
        "http://your-super-jaeger.svc.cluster.local:4317".to_string()
    });

    let service_name =
        std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| env!("CARGO_PKG_NAME").to_string());

    let resource = Resource::builder()
        .with_attribute(KeyValue::new(SERVICE_NAME, service_name))
        .with_attribute(KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION")))
        .build();

    let exporter = SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()
        .expect("Failed to build OTLP span exporter");

    let provider = SdkTracerProvider::builder()
        .with_resource(resource)
        .with_batch_exporter(exporter)
        .build();

    global::set_tracer_provider(provider.clone());
    provider
}

/// Flushes and shuts down the `TracerProvider` gracefully.
/// Call before process exit to ensure all pending spans are exported.
pub fn shutdown_tracer(provider: &SdkTracerProvider) {
    if let Err(e) = provider.shutdown() {
        tracing::error!("Error shutting down tracer provider: {:?}", e);
    }
}
