use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

const BUILD_VERSION_JSON: &str = include_str!("../../build-version.json");
static BUILD_IDENTITY: OnceLock<BuildIdentity> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildIdentity {
    pub schema_version: u32,
    pub product_name: String,
    pub edition: String,
    pub semantic_version: String,
    pub release_cycle: String,
    pub display_build: String,
    pub display_name: String,
    pub release_tag: String,
}

pub fn current() -> &'static BuildIdentity {
    BUILD_IDENTITY.get_or_init(|| {
        let identity: BuildIdentity = serde_json::from_str(BUILD_VERSION_JSON)
            .expect("build-version.json must contain a valid build identity");
        assert_eq!(
            identity.schema_version, 1,
            "unsupported build identity schema"
        );
        assert_eq!(
            identity.semantic_version,
            env!("CARGO_PKG_VERSION"),
            "Cargo package version must match build-version.json"
        );
        identity
    })
}

#[tauri::command]
pub fn get_current_build_info() -> serde_json::Value {
    let identity = current();
    serde_json::json!({
        "name": identity.product_name,
        "version": identity.semantic_version,
        "build": identity.display_build,
        "displayName": identity.display_name,
        "releaseCycle": identity.release_cycle,
        "releaseTag": identity.release_tag,
        "edition": identity.edition,
        "embeddedServer": true,
        "defaultServerPort": 32400,
        "automaticNatTraversal": true,
        "cloudRelayFallback": true,
        "encryptedRemoteTransport": true,
        "opaqueRemoteMediaKeys": true,
        "aiMediaAutopilot": true,
        "spatialExperienceShell": true
    })
}

#[cfg(test)]
mod tests {
    use super::current;

    #[test]
    fn build_identity_is_complete_and_version_aligned() {
        let identity = current();
        assert_eq!(identity.schema_version, 1);
        assert_eq!(identity.semantic_version, env!("CARGO_PKG_VERSION"));
        assert!(identity.display_name.starts_with('v'));
        assert!(identity.release_tag.starts_with('v'));
        assert!(!identity.display_build.trim().is_empty());
    }
}
