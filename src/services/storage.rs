use crate::services::errors::StorageError;
use aws_sdk_s3::error::ProvideErrorMetadata;
use dotenvy::dotenv;

/// Service de stockage avec AWS S3.
///
/// # Pourquoi ce service ?
/// - Découple le contrôleur du système de fichiers local.
/// - Utilise AWS S3 comme backend de stockage.
/// - Permet une meilleure scalabilité et une gestion distribuée des fichiers.
pub struct StorageService;

impl StorageService {
    /// Crée un client AWS S3 à partir des variables d'environnement.
    async fn create_client() -> Result<aws_sdk_s3::Client, StorageError> {
        dotenv().ok();

        // Charger les credentials S3 custom
        let access_key =
            std::env::var("S3_ACCESS_KEY").unwrap_or_else(|_| "minioadmin".to_string());
        let secret_key =
            std::env::var("S3_SECRET_KEY").unwrap_or_else(|_| "minioadmin".to_string());
        let region = std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string());

        // Créer les credentials
        let creds = aws_sdk_s3::config::Credentials::new(
            access_key,
            secret_key,
            None,
            None,
            "custom-provider",
        );

        // Charger la configuration AWS de base
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_sdk_s3::config::Region::new(region))
            .load()
            .await;

        // Builder pour S3
        let mut s3_builder = aws_sdk_s3::config::Builder::from(&config).credentials_provider(creds);

        // Si un endpoint custom est fourni (pour MinIO ou autre S3-compatible)
        if let Ok(endpoint_url) = std::env::var("S3_ENDPOINT") {
            s3_builder = s3_builder.endpoint_url(endpoint_url).force_path_style(true);
        }

        let s3_config = s3_builder.build();
        Ok(aws_sdk_s3::Client::from_conf(s3_config))
    }

    /// Valide la clé S3 pour prévenir les traversées de répertoires.
    fn validate_key(key: &str) -> Result<(), StorageError> {
        if key.contains("..") {
            return Err(StorageError::NotFound("Invalid key".to_string()));
        }
        Ok(())
    }

    /// Convertit une erreur S3 en `StorageError`.
    fn map_s3_error(
        e: &aws_sdk_s3::error::SdkError<aws_sdk_s3::operation::get_object::GetObjectError>,
        key: &str,
    ) -> StorageError {
        let error_msg = e.to_string();
        tracing::debug!("S3 error details: {}", error_msg);

        let is_not_found = if let Some(service_err) = e.as_service_error() {
            service_err.code() == Some("NoSuchKey") || service_err.code() == Some("404")
        } else {
            false
        };

        let error_contains_not_found = error_msg.contains("NoSuchKey")
            || error_msg.contains("404")
            || error_msg.contains("NotFound")
            || error_msg.contains("The specified key does not exist")
            || error_msg.contains("not found");

        if is_not_found || error_contains_not_found {
            tracing::debug!("File not found: {}", key);
            StorageError::NotFound(format!("File not found: {key}"))
        } else {
            StorageError::IoError(format!("S3 error: {error_msg}"))
        }
    }

    /// Récupère et lit le corps de la réponse S3 en bytes.
    async fn collect_body(
        response: aws_sdk_s3::operation::get_object::GetObjectOutput,
    ) -> Result<Vec<u8>, StorageError> {
        let bytes = response
            .body
            .collect()
            .await
            .map_err(|e| StorageError::IoError(format!("Failed to read response body: {e}")))?
            .into_bytes();

        if bytes.is_empty() {
            return Err(StorageError::IoError("Retrieved file is empty".to_string()));
        }

        Ok(bytes.to_vec())
    }

    /// Récupère un fichier depuis AWS S3.
    ///
    /// # Arguments
    /// * `key` - La clé (nom du fichier) à récupérer (ex: "`icons/icon_ampoule.png`").
    ///
    /// # Errors
    /// * `StorageError::NotFound` - Si le fichier n'est pas trouvé.
    /// * `StorageError::IoError` - Si une erreur réseau ou S3 se produit.
    #[tracing::instrument(fields(s3.key = %key))]
    pub async fn get_image(key: &str) -> Result<Vec<u8>, StorageError> {
        dotenv().ok();

        Self::validate_key(key)?;

        let bucket = std::env::var("S3_BUCKET").unwrap_or_else(|_| "images".to_string());
        let client = Self::create_client()
            .await
            .map_err(|e| StorageError::IoError(format!("Failed to create S3 client: {e}")))?;

        let response = client
            .get_object()
            .bucket(&bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| Self::map_s3_error(&e, key))?;

        Self::collect_body(response).await
    }
}
