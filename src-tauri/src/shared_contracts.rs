use crate::metadata_provider_config::MetadataProviderRegistry;
use serde::{Deserialize, Serialize};

pub const SHARED_CONTRACT_VERSION: u32 = 1;
pub const METADATA_PROVIDER_FIXTURE_SHA256: &str =
    "b7ca1f8748296ce7651d17dec3165ac8a37e3aca321eaf558199299b44b5820d";
pub const ARTWORK_CACHE_FIXTURE_SHA256: &str =
    "d9b08d61cd3451278315102da031d0834db315639d49d7c16efb533ddd26e697";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataProviderContract {
    pub id: String,
    pub name: String,
    pub category: String,
    pub enabled: bool,
    pub requires_key: bool,
    pub implemented: bool,
    pub endpoint: Option<String>,
    pub custom_endpoint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataProviderRegistryContract {
    pub schema_version: u32,
    pub policy: String,
    pub credentials_storage: String,
    pub portable_across_operating_systems: bool,
    pub providers: Vec<MetadataProviderContract>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtworkCacheEntryContract {
    pub schema_version: u32,
    pub media_key: String,
    pub kind: String,
    pub mime_type: String,
    pub byte_length: u64,
    pub sha256: String,
    pub width: u32,
    pub height: u32,
    pub source_provider: String,
    pub cache_state: String,
    pub delivery_path: String,
    pub local_path_exposed: bool,
    pub expires_at: Option<String>,
}

pub trait MetadataProviderRegistryInterface {
    fn metadata_provider_contract(&self) -> MetadataProviderRegistryContract;
}

pub trait ArtworkCacheInterface {
    fn artwork_contract(&self) -> ArtworkCacheEntryContract;
}

impl MetadataProviderRegistryInterface for MetadataProviderRegistry {
    fn metadata_provider_contract(&self) -> MetadataProviderRegistryContract {
        MetadataProviderRegistryContract {
            schema_version: SHARED_CONTRACT_VERSION,
            policy: "all_providers_enabled".to_string(),
            credentials_storage: "native_secure_store".to_string(),
            portable_across_operating_systems: true,
            providers: self
                .providers
                .iter()
                .map(|provider| MetadataProviderContract {
                    id: provider.id.clone(),
                    name: provider.name.clone(),
                    category: provider.category.clone(),
                    enabled: provider.enabled,
                    requires_key: provider.requires_key,
                    implemented: provider.implemented,
                    endpoint: provider.endpoint.clone(),
                    custom_endpoint: provider.custom_endpoint.clone(),
                })
                .collect(),
        }
    }
}

impl ArtworkCacheInterface for ArtworkCacheEntryContract {
    fn artwork_contract(&self) -> ArtworkCacheEntryContract {
        self.clone()
    }
}

pub fn validate_metadata_provider_contract(
    contract: &MetadataProviderRegistryContract,
) -> Result<(), String> {
    if contract.schema_version != SHARED_CONTRACT_VERSION {
        return Err(format!(
            "Unsupported metadata provider contract version: {}",
            contract.schema_version
        ));
    }
    if contract.policy != "all_providers_enabled" {
        return Err("Metadata provider policy must enable all providers".to_string());
    }
    if contract.credentials_storage != "native_secure_store" {
        return Err("Credentials must remain in the native secure store".to_string());
    }
    if !contract.portable_across_operating_systems {
        return Err("Metadata provider registry must be cross-platform portable".to_string());
    }
    if contract.providers.is_empty() {
        return Err("Metadata provider registry cannot be empty".to_string());
    }
    if contract.providers.iter().any(|provider| !provider.enabled) {
        return Err("Every metadata provider must remain enabled".to_string());
    }
    Ok(())
}

pub fn validate_artwork_contract(contract: &ArtworkCacheEntryContract) -> Result<(), String> {
    if contract.schema_version != SHARED_CONTRACT_VERSION {
        return Err(format!(
            "Unsupported artwork contract version: {}",
            contract.schema_version
        ));
    }
    if contract.media_key.trim().is_empty() {
        return Err("Artwork media key is required".to_string());
    }
    if !matches!(contract.kind.as_str(), "poster" | "backdrop" | "thumbnail") {
        return Err("Unsupported artwork kind".to_string());
    }
    if !contract.mime_type.starts_with("image/") {
        return Err("Artwork MIME type must be an image".to_string());
    }
    if contract.byte_length == 0 || contract.byte_length > 25 * 1024 * 1024 {
        return Err("Artwork byte length is outside the supported range".to_string());
    }
    if contract.sha256.len() != 64
        || !contract
            .sha256
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return Err("Artwork SHA-256 must be a 64-character hexadecimal value".to_string());
    }
    if contract.width == 0 || contract.height == 0 {
        return Err("Artwork dimensions must be positive".to_string());
    }
    if contract.local_path_exposed {
        return Err("Artwork contracts must not expose local filesystem paths".to_string());
    }
    if !contract.delivery_path.starts_with("/api/artwork/") {
        return Err("Artwork delivery path must use the secured artwork API".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    const METADATA_FIXTURE: &str =
        include_str!("../../contracts/v1/golden/metadata-provider-registry.json");
    const ARTWORK_FIXTURE: &str =
        include_str!("../../contracts/v1/golden/artwork-cache-entry.json");

    fn sha256(value: &str) -> String {
        // Git may materialize text fixtures with CRLF on Windows. The contract hash
        // represents the fixture content, not the checkout platform's line endings.
        let canonical = value.replace("\r\n", "\n").replace('\r', "\n");
        Sha256::digest(canonical.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    #[test]
    fn metadata_provider_fixture_hash_is_canonical() {
        assert_eq!(sha256(METADATA_FIXTURE), METADATA_PROVIDER_FIXTURE_SHA256);
    }

    #[test]
    fn artwork_fixture_hash_is_canonical() {
        assert_eq!(sha256(ARTWORK_FIXTURE), ARTWORK_CACHE_FIXTURE_SHA256);
    }

    #[test]
    fn metadata_provider_contract_round_trips_golden_json() {
        let decoded: MetadataProviderRegistryContract =
            serde_json::from_str(METADATA_FIXTURE).expect("metadata fixture decodes");
        validate_metadata_provider_contract(&decoded).expect("metadata contract validates");
        let encoded = serde_json::to_value(decoded).expect("metadata contract encodes");
        let golden: serde_json::Value =
            serde_json::from_str(METADATA_FIXTURE).expect("metadata fixture is valid JSON");
        assert_eq!(encoded, golden);
    }

    #[test]
    fn artwork_contract_round_trips_golden_json() {
        let decoded: ArtworkCacheEntryContract =
            serde_json::from_str(ARTWORK_FIXTURE).expect("artwork fixture decodes");
        validate_artwork_contract(&decoded).expect("artwork contract validates");
        let encoded = serde_json::to_value(decoded).expect("artwork contract encodes");
        let golden: serde_json::Value =
            serde_json::from_str(ARTWORK_FIXTURE).expect("artwork fixture is valid JSON");
        assert_eq!(encoded, golden);
    }

    #[test]
    fn contract_rejects_local_path_exposure() {
        let mut decoded: ArtworkCacheEntryContract =
            serde_json::from_str(ARTWORK_FIXTURE).expect("artwork fixture decodes");
        decoded.local_path_exposed = true;
        assert!(validate_artwork_contract(&decoded).is_err());
    }
}
