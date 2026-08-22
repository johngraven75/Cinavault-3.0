use axum::{
    extract::{Path as RoutePath, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{self, Write},
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

mod volume_probe;
pub use volume_probe::{VolumeIdentity, VolumeIdentityKind};

pub const SERVICE_NAME: &str = "CinaVault 3.0 Service Foundation";
pub const CONTRACT_VERSION: &str = "v3alpha3";
const REGISTRY_SCHEMA_VERSION: u16 = 1;
const INSPECTION_ENTRY_LIMIT: usize = 100;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VolumeKind {
    Smb,
    Nfs,
    Local,
    Iscsi,
    Removable,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PowerPolicy {
    AlwaysOn,
    SpinsDown,
    Removable,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VolumeHealth {
    Online,
    Offline,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SentinelStatus {
    Verified,
    DerivedReadOnly,
    Missing,
    Mismatch,
    NotChecked,
}

impl SentinelStatus {
    fn permits_reconcile(&self) -> bool {
        matches!(self, Self::Verified | Self::DerivedReadOnly)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct VolumeRoute {
    pub path: String,
    pub priority: u16,
    pub healthy: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Volume {
    pub id: String,
    pub label: String,
    pub kind: VolumeKind,
    pub routes: Vec<VolumeRoute>,
    pub health: VolumeHealth,
    pub sentinel_status: SentinelStatus,
    pub read_only: bool,
    pub power_policy: PowerPolicy,
    pub last_spin_up_cause: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReconcileOutcome {
    ReadyDryRun,
    Offline,
    AbortedUnverifiedVolume,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReconcileRequest {
    pub volume: Volume,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReconcilePlan {
    pub outcome: ReconcileOutcome,
    pub dry_run: bool,
    pub changes: Vec<String>,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InspectionStatus {
    Ready,
    Offline,
    Unverified,
    Unavailable,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceEntryKind {
    File,
    Directory,
    Symlink,
    Other,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceEntrySample {
    pub name: String,
    pub kind: SourceEntryKind,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceInspection {
    pub volume_id: String,
    pub status: InspectionStatus,
    pub route: Option<String>,
    pub entries: Vec<SourceEntrySample>,
    pub truncated: bool,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProbeStatus {
    Ready,
    Offline,
    Unverified,
    Unavailable,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct VolumeProbe {
    pub volume_id: String,
    pub status: ProbeStatus,
    pub route: Option<String>,
    pub identity: Option<VolumeIdentity>,
    pub message: String,
}

#[derive(Debug)]
pub enum RegistryError {
    Invalid(String),
    Duplicate(String),
    Io(String),
    Decode(String),
    UnsupportedSchema(u16),
    NotFound(String),
}

#[derive(Clone)]
pub struct AppState {
    service_version: String,
    registry: Arc<RwLock<VolumeRegistry>>,
}

impl AppState {
    pub fn with_registry_path(path: impl Into<PathBuf>) -> Result<Self, RegistryError> {
        Ok(Self {
            service_version: env!("CARGO_PKG_VERSION").to_owned(),
            registry: Arc::new(RwLock::new(VolumeRegistry::load(path.into())?)),
        })
    }

    fn in_memory() -> Self {
        Self {
            service_version: env!("CARGO_PKG_VERSION").to_owned(),
            registry: Arc::new(RwLock::new(VolumeRegistry::in_memory())),
        }
    }

    fn load_default() -> Result<Self, RegistryError> {
        Self::with_registry_path(default_registry_path())
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::in_memory()
    }
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    service: &'static str,
    version: String,
    contract_version: &'static str,
    bind_policy: &'static str,
}

#[derive(Debug, Serialize)]
struct ApiError {
    code: &'static str,
    message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PersistedRegistry {
    schema_version: u16,
    volumes: Vec<Volume>,
}

struct VolumeRegistry {
    path: Option<PathBuf>,
    volumes: BTreeMap<String, Volume>,
}

impl VolumeRegistry {
    fn in_memory() -> Self {
        Self {
            path: None,
            volumes: BTreeMap::new(),
        }
    }

    fn load(path: PathBuf) -> Result<Self, RegistryError> {
        let snapshot = match read_registry_snapshot(&path) {
            Ok(snapshot) => snapshot,
            Err(_) if !path.exists() => PersistedRegistry {
                schema_version: REGISTRY_SCHEMA_VERSION,
                volumes: Vec::new(),
            },
            Err(primary_error) => {
                let backup_path = registry_backup_path(&path);
                if backup_path.exists() {
                    read_registry_snapshot(&backup_path).map_err(|backup_error| {
                        RegistryError::Decode(format!(
                            "primary registry could not be read ({primary_error:?}); backup could not be read ({backup_error:?})"
                        ))
                    })?
                } else {
                    return Err(primary_error);
                }
            }
        };

        if snapshot.schema_version != REGISTRY_SCHEMA_VERSION {
            return Err(RegistryError::UnsupportedSchema(snapshot.schema_version));
        }

        let mut volumes = BTreeMap::new();
        for volume in snapshot.volumes {
            let normalized = normalize_volume(volume);
            validate_volume(&normalized)?;
            if volumes.insert(normalized.id.clone(), normalized).is_some() {
                return Err(RegistryError::Decode(
                    "registry contains duplicate volume identifiers".to_owned(),
                ));
            }
        }

        Ok(Self {
            path: Some(path),
            volumes,
        })
    }

    fn list(&self) -> Vec<Volume> {
        self.volumes.values().cloned().collect()
    }

    fn register(&mut self, volume: Volume) -> Result<Volume, RegistryError> {
        let normalized = normalize_volume(volume);
        validate_volume(&normalized)?;
        if self.volumes.contains_key(&normalized.id) {
            return Err(RegistryError::Duplicate(normalized.id));
        }

        let mut next = self.volumes.clone();
        next.insert(normalized.id.clone(), normalized.clone());
        self.persist(&next)?;
        self.volumes = next;
        Ok(normalized)
    }

    fn inspect(&self, volume_id: &str) -> Result<SourceInspection, RegistryError> {
        let volume = self
            .volumes
            .get(volume_id)
            .ok_or_else(|| RegistryError::NotFound(volume_id.to_owned()))?;

        if volume.health == VolumeHealth::Offline {
            return Ok(SourceInspection {
                volume_id: volume.id.clone(),
                status: InspectionStatus::Offline,
                route: None,
                entries: Vec::new(),
                truncated: false,
                message: "Volume is marked offline; inspection is skipped and no source state is changed."
                    .to_owned(),
            });
        }

        if !volume.sentinel_status.permits_reconcile() {
            return Ok(SourceInspection {
                volume_id: volume.id.clone(),
                status: InspectionStatus::Unverified,
                route: None,
                entries: Vec::new(),
                truncated: false,
                message: "Volume identity is unverified; inspection is skipped before reading a source route."
                    .to_owned(),
            });
        }

        let route = volume
            .routes
            .iter()
            .filter(|route| route.healthy)
            .min_by_key(|route| route.priority);
        let Some(route) = route else {
            return Ok(SourceInspection {
                volume_id: volume.id.clone(),
                status: InspectionStatus::Unavailable,
                route: None,
                entries: Vec::new(),
                truncated: false,
                message: "No healthy registered route is available for this volume.".to_owned(),
            });
        };

        let source_path = Path::new(&route.path);
        let entries = match read_immediate_entries(source_path) {
            Ok(entries) => entries,
            Err(error) => {
                return Ok(SourceInspection {
                    volume_id: volume.id.clone(),
                    status: InspectionStatus::Unavailable,
                    route: Some(route.path.clone()),
                    entries: Vec::new(),
                    truncated: false,
                    message: format!("Registered route is unavailable or unreadable: {error}"),
                });
            }
        };

        let truncated = entries.len() > INSPECTION_ENTRY_LIMIT;
        let entries = entries.into_iter().take(INSPECTION_ENTRY_LIMIT).collect();
        Ok(SourceInspection {
            volume_id: volume.id.clone(),
            status: InspectionStatus::Ready,
            route: Some(route.path.clone()),
            entries,
            truncated,
            message: "Read-only immediate-entry inspection completed; no recursive scan or catalogue write was performed."
                .to_owned(),
        })
    }

    fn probe(&self, volume_id: &str) -> Result<VolumeProbe, RegistryError> {
        let volume = self
            .volumes
            .get(volume_id)
            .ok_or_else(|| RegistryError::NotFound(volume_id.to_owned()))?;

        if volume.health == VolumeHealth::Offline {
            return Ok(VolumeProbe {
                volume_id: volume.id.clone(),
                status: ProbeStatus::Offline,
                route: None,
                identity: None,
                message: "Volume is marked offline; route probing is skipped and registry state is unchanged."
                    .to_owned(),
            });
        }

        if !volume.sentinel_status.permits_reconcile() {
            return Ok(VolumeProbe {
                volume_id: volume.id.clone(),
                status: ProbeStatus::Unverified,
                route: None,
                identity: None,
                message: "Volume identity is unverified; route probing is skipped before accessing a source route."
                    .to_owned(),
            });
        }

        let route = volume
            .routes
            .iter()
            .filter(|route| route.healthy)
            .min_by_key(|route| route.priority);
        let Some(route) = route else {
            return Ok(VolumeProbe {
                volume_id: volume.id.clone(),
                status: ProbeStatus::Unavailable,
                route: None,
                identity: None,
                message: "No healthy registered route is available for this volume.".to_owned(),
            });
        };

        match volume_probe::probe_route(Path::new(&route.path)) {
            Ok(identity) => Ok(VolumeProbe {
                volume_id: volume.id.clone(),
                status: ProbeStatus::Ready,
                route: Some(route.path.clone()),
                identity: Some(identity),
                message: "Read-only route probe completed; no registry, source, or catalogue state was changed."
                    .to_owned(),
            }),
            Err(error) => Ok(VolumeProbe {
                volume_id: volume.id.clone(),
                status: ProbeStatus::Unavailable,
                route: Some(route.path.clone()),
                identity: None,
                message: format!("Registered route is unavailable or unreadable: {error}"),
            }),
        }
    }

    fn persist(&self, volumes: &BTreeMap<String, Volume>) -> Result<(), RegistryError> {
        let Some(path) = &self.path else {
            return Err(RegistryError::Io(
                "the in-memory registry cannot accept durable registrations".to_owned(),
            ));
        };
        let parent = path.parent().ok_or_else(|| {
            RegistryError::Io("registry path must have a parent directory".to_owned())
        })?;
        fs::create_dir_all(parent).map_err(|error| RegistryError::Io(error.to_string()))?;

        let snapshot = PersistedRegistry {
            schema_version: REGISTRY_SCHEMA_VERSION,
            volumes: volumes.values().cloned().collect(),
        };
        let bytes = serde_json::to_vec_pretty(&snapshot)
            .map_err(|error| RegistryError::Io(error.to_string()))?;
        let staging = registry_staging_path(path);
        let backup = registry_backup_path(path);

        write_synced_file(&staging, &bytes)?;
        if path.exists() {
            fs::copy(path, &backup).map_err(|error| RegistryError::Io(error.to_string()))?;
            File::open(&backup)
                .and_then(|file| file.sync_all())
                .map_err(|error| RegistryError::Io(error.to_string()))?;
            fs::remove_file(path).map_err(|error| RegistryError::Io(error.to_string()))?;
        }
        fs::rename(&staging, path).map_err(|error| RegistryError::Io(error.to_string()))?;
        Ok(())
    }
}

fn read_registry_snapshot(path: &Path) -> Result<PersistedRegistry, RegistryError> {
    let bytes = fs::read(path).map_err(|error| RegistryError::Io(error.to_string()))?;
    serde_json::from_slice(&bytes).map_err(|error| RegistryError::Decode(error.to_string()))
}

fn write_synced_file(path: &Path, bytes: &[u8]) -> Result<(), RegistryError> {
    let mut file = File::create(path).map_err(|error| RegistryError::Io(error.to_string()))?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| RegistryError::Io(error.to_string()))
}

fn registry_staging_path(path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.new", path.display()))
}

fn registry_backup_path(path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.bak", path.display()))
}

fn default_registry_path() -> PathBuf {
    if let Ok(value) = std::env::var("CINAVAULT_SERVER_DATA_DIR") {
        if !value.trim().is_empty() {
            return PathBuf::from(value).join("volumes.v1.json");
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(value) = std::env::var("LOCALAPPDATA") {
            return PathBuf::from(value)
                .join("CinaVault 3.0")
                .join("Service")
                .join("volumes.v1.json");
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(value) = std::env::var("XDG_STATE_HOME") {
            return PathBuf::from(value)
                .join("cinavault-3")
                .join("service")
                .join("volumes.v1.json");
        }
        if let Ok(value) = std::env::var("HOME") {
            return PathBuf::from(value)
                .join(".local")
                .join("state")
                .join("cinavault-3")
                .join("service")
                .join("volumes.v1.json");
        }
    }

    std::env::temp_dir()
        .join("cinavault-3")
        .join("service")
        .join("volumes.v1.json")
}

fn normalize_volume(mut volume: Volume) -> Volume {
    volume.id = volume.id.trim().to_owned();
    volume.label = volume.label.trim().to_owned();
    volume.last_spin_up_cause = volume
        .last_spin_up_cause
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    for route in &mut volume.routes {
        route.path = route.path.trim().to_owned();
    }
    volume
}

fn validate_volume(volume: &Volume) -> Result<(), RegistryError> {
    if volume.id.is_empty()
        || volume.id.len() > 128
        || !volume
            .id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err(RegistryError::Invalid(
            "volume id must contain only ASCII letters, numbers, hyphens, or underscores"
                .to_owned(),
        ));
    }
    if volume.label.is_empty() || volume.label.len() > 256 {
        return Err(RegistryError::Invalid(
            "volume label must be between 1 and 256 characters".to_owned(),
        ));
    }
    if volume.routes.is_empty() {
        return Err(RegistryError::Invalid(
            "at least one registered source route is required".to_owned(),
        ));
    }

    let mut priorities = BTreeSet::new();
    let mut paths = BTreeSet::new();
    for route in &volume.routes {
        if route.priority == 0 || !priorities.insert(route.priority) {
            return Err(RegistryError::Invalid(
                "route priorities must be unique positive integers".to_owned(),
            ));
        }
        if route.path.is_empty() || !paths.insert(route.path.to_ascii_lowercase()) {
            return Err(RegistryError::Invalid(
                "registered routes must be non-empty and unique".to_owned(),
            ));
        }
        if matches!(volume.kind, VolumeKind::Smb) && !route.path.starts_with(r"\\") {
            return Err(RegistryError::Invalid(
                "SMB volume routes must use canonical UNC paths beginning with \\\\".to_owned(),
            ));
        }
        if !matches!(volume.kind, VolumeKind::Smb) && !looks_absolute(&route.path) {
            return Err(RegistryError::Invalid(
                "local, NFS, iSCSI, and removable routes must be absolute paths".to_owned(),
            ));
        }
    }
    Ok(())
}

fn looks_absolute(path: &str) -> bool {
    path.starts_with('/')
        || path.starts_with(r"\\")
        || (path.len() >= 3
            && path.as_bytes()[0].is_ascii_alphabetic()
            && path.as_bytes()[1] == b':'
            && matches!(path.as_bytes()[2], b'\\' | b'/'))
}

fn read_immediate_entries(path: &Path) -> Result<Vec<SourceEntrySample>, String> {
    if !path.is_dir() {
        return Err(
            "path does not exist, is not a directory, or the volume is disconnected".to_owned(),
        );
    }

    let entries = fs::read_dir(path).map_err(|error| error.to_string())?;
    let mut samples = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        let kind = entry.file_type().map_err(|error| error.to_string())?;
        samples.push(SourceEntrySample {
            name: entry.file_name().to_string_lossy().to_string(),
            kind: if kind.is_file() {
                SourceEntryKind::File
            } else if kind.is_dir() {
                SourceEntryKind::Directory
            } else if kind.is_symlink() {
                SourceEntryKind::Symlink
            } else {
                SourceEntryKind::Other
            },
        });
    }
    samples.sort_by(|left, right| {
        left.name
            .to_ascii_lowercase()
            .cmp(&right.name.to_ascii_lowercase())
            .then_with(|| left.name.cmp(&right.name))
    });
    Ok(samples)
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route(
            "/Cinevault/Volumes",
            get(list_volumes).post(register_volume),
        )
        .route("/Cinevault/Volumes/:id/Inspect", post(inspect_volume))
        .route("/Cinevault/Volumes/:id/Probe", post(probe_volume))
        .route("/Cinevault/Volumes/ReconcilePlan", post(reconcile_plan))
        .with_state(Arc::new(state))
}

pub async fn serve(bind_address: SocketAddr) -> io::Result<()> {
    let state = AppState::load_default().map_err(registry_error_to_io)?;
    let listener = tokio::net::TcpListener::bind(bind_address).await?;
    axum::serve(listener, build_router(state)).await
}

pub fn create_reconcile_plan(request: &ReconcileRequest) -> ReconcilePlan {
    if request.volume.health == VolumeHealth::Offline {
        return ReconcilePlan {
            outcome: ReconcileOutcome::Offline,
            dry_run: true,
            changes: Vec::new(),
            message:
                "Volume is offline; reconciliation is blocked and no library changes are planned."
                    .to_owned(),
        };
    }

    if !request.volume.sentinel_status.permits_reconcile() {
        return ReconcilePlan {
            outcome: ReconcileOutcome::AbortedUnverifiedVolume,
            dry_run: true,
            changes: Vec::new(),
            message: "Volume identity is not verified; reconciliation is aborted before any destructive action can be considered."
                .to_owned(),
        };
    }

    ReconcilePlan {
        outcome: ReconcileOutcome::ReadyDryRun,
        dry_run: true,
        changes: Vec::new(),
        message: "Volume is verified. This foundation produces a no-write dry-run plan only; scanning and catalogue mutation are not enabled."
            .to_owned(),
    }
}

async fn health(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    Json(HealthResponse {
        service: SERVICE_NAME,
        version: state.service_version.clone(),
        contract_version: CONTRACT_VERSION,
        bind_policy: "loopback by default; explicit bind address required for any other interface",
    })
}

async fn list_volumes(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<Volume>>, (StatusCode, Json<ApiError>)> {
    let registry = state.registry.read().map_err(|_| state_error())?;
    Ok(Json(registry.list()))
}

async fn register_volume(
    State(state): State<Arc<AppState>>,
    Json(volume): Json<Volume>,
) -> Result<(StatusCode, Json<Volume>), (StatusCode, Json<ApiError>)> {
    let mut registry = state.registry.write().map_err(|_| state_error())?;
    let registered = registry.register(volume).map_err(registry_error_response)?;
    Ok((StatusCode::CREATED, Json(registered)))
}

async fn inspect_volume(
    State(state): State<Arc<AppState>>,
    RoutePath(volume_id): RoutePath<String>,
) -> Result<Json<SourceInspection>, (StatusCode, Json<ApiError>)> {
    let registry = state.registry.read().map_err(|_| state_error())?;
    let inspection = registry
        .inspect(&volume_id)
        .map_err(registry_error_response)?;
    Ok(Json(inspection))
}

async fn probe_volume(
    State(state): State<Arc<AppState>>,
    RoutePath(volume_id): RoutePath<String>,
) -> Result<Json<VolumeProbe>, (StatusCode, Json<ApiError>)> {
    let registry = state.registry.read().map_err(|_| state_error())?;
    let probe = registry
        .probe(&volume_id)
        .map_err(registry_error_response)?;
    Ok(Json(probe))
}

async fn reconcile_plan(
    Json(request): Json<ReconcileRequest>,
) -> Result<Json<ReconcilePlan>, (StatusCode, Json<ApiError>)> {
    if !request.dry_run {
        return Err((
            StatusCode::CONFLICT,
            Json(ApiError {
                code: "dry_run_required",
                message: "This service foundation only supports dry-run reconciliation plans."
                    .to_owned(),
            }),
        ));
    }

    Ok(Json(create_reconcile_plan(&request)))
}

fn state_error() -> (StatusCode, Json<ApiError>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ApiError {
            code: "registry_state_unavailable",
            message: "The local volume registry is temporarily unavailable.".to_owned(),
        }),
    )
}

fn registry_error_response(error: RegistryError) -> (StatusCode, Json<ApiError>) {
    match error {
        RegistryError::Invalid(message) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ApiError {
                code: "invalid_volume_registration",
                message,
            }),
        ),
        RegistryError::Duplicate(volume_id) => (
            StatusCode::CONFLICT,
            Json(ApiError {
                code: "duplicate_volume_id",
                message: format!("A volume with id '{volume_id}' is already registered."),
            }),
        ),
        RegistryError::NotFound(volume_id) => (
            StatusCode::NOT_FOUND,
            Json(ApiError {
                code: "volume_not_found",
                message: format!("No registered volume exists with id '{volume_id}'."),
            }),
        ),
        RegistryError::UnsupportedSchema(version) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                code: "unsupported_registry_schema",
                message: format!(
                    "Registry schema version {version} is not supported by this service."
                ),
            }),
        ),
        RegistryError::Io(message) | RegistryError::Decode(message) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                code: "registry_persistence_failed",
                message: format!(
                    "The local volume registry could not be safely updated: {message}"
                ),
            }),
        ),
    }
}

fn registry_error_to_io(error: RegistryError) -> io::Error {
    io::Error::new(
        io::ErrorKind::Other,
        format!("volume registry startup failed: {error:?}"),
    )
}
