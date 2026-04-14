use opentelemetry::{KeyValue, global};
use opentelemetry_otlp::{SpanExporter, WithExportConfig};
use opentelemetry_sdk::{Resource, trace::SdkTracerProvider};
use opentelemetry_semantic_conventions::resource::{SERVICE_NAME, SERVICE_VERSION};

/// Initialise le pipeline OpenTelemetry → OTLP → Jaeger.
///
/// Variables d'environnement :
/// - `OTEL_EXPORTER_OTLP_ENDPOINT` : endpoint gRPC OTLP
///   (défaut : `http://jaeger-collector.ns-sae5-z11.svc.cluster.local:4317`)
/// - `OTEL_SERVICE_NAME` : nom du service dans Jaeger (défaut : `api_s3`)
///
/// Retourne le `SdkTracerProvider` pour permettre un shutdown propre.
///
/// # Panics
/// Panique si l'exporteur OTLP ne peut pas être construit (endpoint invalide, tonic indisponible).
#[must_use]
pub fn init_tracer() -> SdkTracerProvider {
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").unwrap_or_else(|_| {
        "http://jaeger-collector.ns-sae5-z11.svc.cluster.local:4317".to_string()
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

/// Flush et arrête proprement le `TracerProvider`.
/// À appeler avant la fin du processus pour envoyer tous les spans en attente.
pub fn shutdown_tracer(provider: &SdkTracerProvider) {
    if let Err(e) = provider.shutdown() {
        tracing::error!("Error shutting down tracer provider: {:?}", e);
    }
}
