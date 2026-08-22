use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};

pub const SERVICE_NAME: &str = "CinaVault 3.0 Service Foundation";
pub const CONTRACT_VERSION: &str = "v3alpha1";

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

#[derive(Clone)]
pub struct AppState {
    service_version: String,
    volumes: Vec<Volume>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            service_version: env!("CARGO_PKG_VERSION").to_owned(),
            volumes: Vec::new(),
        }
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
    message: &'static str,
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/Cinevault/Volumes", get(list_volumes))
        .route("/Cinevault/Volumes/ReconcilePlan", post(reconcile_plan))
        .with_state(Arc::new(state))
}

pub async fn serve(bind_address: SocketAddr) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind(bind_address).await?;
    axum::serve(listener, build_router(AppState::default())).await
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

async fn list_volumes(State(state): State<Arc<AppState>>) -> Json<Vec<Volume>> {
    Json(state.volumes.clone())
}

async fn reconcile_plan(
    Json(request): Json<ReconcileRequest>,
) -> Result<Json<ReconcilePlan>, (StatusCode, Json<ApiError>)> {
    if !request.dry_run {
        return Err((
            StatusCode::CONFLICT,
            Json(ApiError {
                code: "dry_run_required",
                message: "This service foundation only supports dry-run reconciliation plans.",
            }),
        ));
    }

    Ok(Json(create_reconcile_plan(&request)))
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::BAD_REQUEST, Json(self)).into_response()
    }
}
