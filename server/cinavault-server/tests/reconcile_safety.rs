use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use cinavault_server::{
    build_router, create_reconcile_plan, AppState, PowerPolicy, ReconcileOutcome, ReconcileRequest,
    SentinelStatus, Volume, VolumeHealth, VolumeKind, VolumeRoute,
};
use tower::ServiceExt;

fn volume(health: VolumeHealth, sentinel_status: SentinelStatus) -> Volume {
    Volume {
        id: "volume-001".to_owned(),
        label: "Media NAS".to_owned(),
        kind: VolumeKind::Smb,
        routes: vec![VolumeRoute {
            path: r"\\nas\media".to_owned(),
            priority: 1,
            healthy: health == VolumeHealth::Online,
        }],
        health,
        sentinel_status,
        read_only: false,
        power_policy: PowerPolicy::SpinsDown,
        last_spin_up_cause: None,
    }
}

#[test]
fn missing_sentinel_aborts_before_any_change_is_planned() {
    let plan = create_reconcile_plan(&ReconcileRequest {
        volume: volume(VolumeHealth::Online, SentinelStatus::Missing),
        dry_run: true,
    });

    assert_eq!(plan.outcome, ReconcileOutcome::AbortedUnverifiedVolume);
    assert!(plan.changes.is_empty());
    assert!(plan.dry_run);
}

#[test]
fn offline_volume_never_yields_a_delete_or_purge_plan() {
    let plan = create_reconcile_plan(&ReconcileRequest {
        volume: volume(VolumeHealth::Offline, SentinelStatus::Verified),
        dry_run: true,
    });

    assert_eq!(plan.outcome, ReconcileOutcome::Offline);
    assert!(plan.changes.is_empty());
    assert!(plan.dry_run);
}

#[test]
fn verified_volume_is_still_dry_run_only_in_the_foundation_milestone() {
    let plan = create_reconcile_plan(&ReconcileRequest {
        volume: volume(VolumeHealth::Online, SentinelStatus::Verified),
        dry_run: true,
    });

    assert_eq!(plan.outcome, ReconcileOutcome::ReadyDryRun);
    assert!(plan.changes.is_empty());
    assert!(plan.dry_run);
}

#[tokio::test]
async fn health_endpoint_reports_loopback_only_foundation_policy() {
    let response = build_router(AppState::default())
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(payload["contract_version"], "v3alpha3");
    assert!(payload["bind_policy"]
        .as_str()
        .unwrap()
        .contains("loopback"));
}

#[tokio::test]
async fn reconcile_endpoint_rejects_non_dry_run_requests() {
    let request_json = serde_json::json!({
        "dry_run": false,
        "volume": volume(VolumeHealth::Online, SentinelStatus::Verified),
    });
    let response = build_router(AppState::default())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/Cinevault/Volumes/ReconcilePlan")
                .header("content-type", "application/json")
                .body(Body::from(request_json.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(payload["code"], "dry_run_required");
}

use std::{
    fs,
    path::{Path as FsPath, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_TEMP_DIRECTORY: AtomicU64 = AtomicU64::new(0);

fn temporary_state_directory() -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "cinavault-server-test-{}-{}",
        std::process::id(),
        NEXT_TEMP_DIRECTORY.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&directory).unwrap();
    directory
}

fn local_volume(
    id: &str,
    root: &FsPath,
    health: VolumeHealth,
    sentinel_status: SentinelStatus,
) -> Volume {
    Volume {
        id: id.to_owned(),
        label: format!("{id} local media"),
        kind: VolumeKind::Local,
        routes: vec![VolumeRoute {
            path: root.to_string_lossy().to_string(),
            priority: 1,
            healthy: true,
        }],
        health,
        sentinel_status,
        read_only: true,
        power_policy: PowerPolicy::AlwaysOn,
        last_spin_up_cause: None,
    }
}

async fn register_volume_via_api(state: AppState, volume: Volume) -> StatusCode {
    build_router(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/Cinevault/Volumes")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&volume).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
}

#[tokio::test]
async fn registered_volume_survives_service_state_restart() {
    let state_directory = temporary_state_directory();
    let registry_path = state_directory.join("volumes.v1.json");
    let source_directory = state_directory.join("source");
    fs::create_dir_all(&source_directory).unwrap();
    let volume = local_volume(
        "local-library",
        &source_directory,
        VolumeHealth::Online,
        SentinelStatus::Verified,
    );

    let status = register_volume_via_api(
        AppState::with_registry_path(&registry_path).unwrap(),
        volume,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert!(registry_path.exists());

    let response = build_router(AppState::with_registry_path(&registry_path).unwrap())
        .oneshot(
            Request::builder()
                .uri("/Cinevault/Volumes")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let volumes: Vec<Volume> = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(volumes.len(), 1);
    assert_eq!(volumes[0].id, "local-library");

    fs::remove_dir_all(state_directory).unwrap();
}

#[tokio::test]
async fn duplicate_volume_registration_is_rejected_without_mutating_registry() {
    let state_directory = temporary_state_directory();
    let registry_path = state_directory.join("volumes.v1.json");
    let source_directory = state_directory.join("source");
    fs::create_dir_all(&source_directory).unwrap();
    let volume = local_volume(
        "duplicate-test",
        &source_directory,
        VolumeHealth::Online,
        SentinelStatus::Verified,
    );

    let state = AppState::with_registry_path(&registry_path).unwrap();
    assert_eq!(
        register_volume_via_api(state.clone(), volume.clone()).await,
        StatusCode::CREATED
    );
    assert_eq!(
        register_volume_via_api(state, volume).await,
        StatusCode::CONFLICT
    );

    let state_after_restart = AppState::with_registry_path(&registry_path).unwrap();
    let response = build_router(state_after_restart)
        .oneshot(
            Request::builder()
                .uri("/Cinevault/Volumes")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let volumes: Vec<Volume> = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(volumes.len(), 1);

    fs::remove_dir_all(state_directory).unwrap();
}

#[tokio::test]
async fn verified_registered_volume_produces_a_bounded_non_recursive_sample() {
    let state_directory = temporary_state_directory();
    let registry_path = state_directory.join("volumes.v1.json");
    let source_directory = state_directory.join("source");
    fs::create_dir_all(source_directory.join("nested")).unwrap();
    fs::write(
        source_directory.join("zeta.mkv"),
        b"not-opened-by-inspection",
    )
    .unwrap();
    fs::write(
        source_directory.join("alpha.mp4"),
        b"not-opened-by-inspection",
    )
    .unwrap();
    fs::write(
        source_directory.join("nested").join("hidden.mkv"),
        b"nested",
    )
    .unwrap();
    let volume = local_volume(
        "inspect-ready",
        &source_directory,
        VolumeHealth::Online,
        SentinelStatus::Verified,
    );

    let state = AppState::with_registry_path(&registry_path).unwrap();
    assert_eq!(
        register_volume_via_api(state.clone(), volume).await,
        StatusCode::CREATED
    );
    let response = build_router(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/Cinevault/Volumes/inspect-ready/Inspect")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let inspection: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(inspection["status"], "ready");
    assert_eq!(inspection["entries"][0]["name"], "alpha.mp4");
    assert_eq!(inspection["entries"][1]["name"], "nested");
    assert_eq!(inspection["entries"][2]["name"], "zeta.mkv");
    assert!(!inspection["entries"].to_string().contains("hidden.mkv"));

    fs::remove_dir_all(state_directory).unwrap();
}

#[tokio::test]
async fn offline_and_unverified_registered_volumes_never_read_source_entries() {
    for (volume_id, health, sentinel, expected_status) in [
        (
            "offline-volume",
            VolumeHealth::Offline,
            SentinelStatus::Verified,
            "offline",
        ),
        (
            "unverified-volume",
            VolumeHealth::Online,
            SentinelStatus::Missing,
            "unverified",
        ),
    ] {
        let state_directory = temporary_state_directory();
        let registry_path = state_directory.join("volumes.v1.json");
        let source_directory = state_directory.join("source");
        fs::create_dir_all(&source_directory).unwrap();
        fs::write(
            source_directory.join("would-be-visible.mkv"),
            b"do-not-read",
        )
        .unwrap();
        let volume = local_volume(volume_id, &source_directory, health, sentinel);
        let state = AppState::with_registry_path(&registry_path).unwrap();
        assert_eq!(
            register_volume_via_api(state.clone(), volume).await,
            StatusCode::CREATED
        );

        let response = build_router(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/Cinevault/Volumes/{volume_id}/Inspect"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let inspection: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(inspection["status"], expected_status);
        assert!(inspection["entries"].as_array().unwrap().is_empty());

        fs::remove_dir_all(state_directory).unwrap();
    }
}

#[tokio::test]
async fn smb_registration_requires_a_canonical_unc_route() {
    let state_directory = temporary_state_directory();
    let registry_path = state_directory.join("volumes.v1.json");
    let mut invalid_smb = volume(VolumeHealth::Online, SentinelStatus::Verified);
    invalid_smb.id = "bad-smb".to_owned();
    invalid_smb.routes[0].path = "/not/a/unc/path".to_owned();

    let response = build_router(AppState::with_registry_path(&registry_path).unwrap())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/Cinevault/Volumes")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&invalid_smb).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    fs::remove_dir_all(state_directory).unwrap();
}

#[tokio::test]
async fn verified_registered_volume_reports_transient_route_identity_without_registry_mutation() {
    let state_directory = temporary_state_directory();
    let registry_path = state_directory.join("volumes.v1.json");
    let source_directory = state_directory.join("source");
    fs::create_dir_all(&source_directory).unwrap();
    let volume = local_volume(
        "probe-ready",
        &source_directory,
        VolumeHealth::Online,
        SentinelStatus::Verified,
    );
    let state = AppState::with_registry_path(&registry_path).unwrap();
    assert_eq!(
        register_volume_via_api(state.clone(), volume).await,
        StatusCode::CREATED
    );
    let registry_before = fs::read(&registry_path).unwrap();

    let response = build_router(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/Cinevault/Volumes/probe-ready/Probe")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let probe: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(probe["status"], "ready");
    assert_eq!(
        probe["route"],
        source_directory.to_string_lossy().to_string()
    );
    assert!(probe["identity"]["value"]
        .as_str()
        .unwrap()
        .starts_with("path:"));
    assert_eq!(probe["identity"]["kind"], "path_fingerprint");
    assert_eq!(fs::read(&registry_path).unwrap(), registry_before);

    fs::remove_dir_all(state_directory).unwrap();
}

#[tokio::test]
async fn offline_and_unverified_volumes_skip_probe_before_reading_route() {
    for (volume_id, health, sentinel, expected_status) in [
        (
            "probe-offline",
            VolumeHealth::Offline,
            SentinelStatus::Verified,
            "offline",
        ),
        (
            "probe-unverified",
            VolumeHealth::Online,
            SentinelStatus::Missing,
            "unverified",
        ),
    ] {
        let state_directory = temporary_state_directory();
        let registry_path = state_directory.join("volumes.v1.json");
        let unavailable_source = state_directory.join("not-present");
        let volume = local_volume(volume_id, &unavailable_source, health, sentinel);
        let state = AppState::with_registry_path(&registry_path).unwrap();
        assert_eq!(
            register_volume_via_api(state.clone(), volume).await,
            StatusCode::CREATED
        );

        let response = build_router(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/Cinevault/Volumes/{volume_id}/Probe"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let probe: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(probe["status"], expected_status);
        assert!(probe["route"].is_null());
        assert!(probe["identity"].is_null());

        fs::remove_dir_all(state_directory).unwrap();
    }
}

#[tokio::test]
async fn unavailable_and_unknown_volumes_return_safe_probe_responses() {
    let state_directory = temporary_state_directory();
    let registry_path = state_directory.join("volumes.v1.json");
    let unavailable_source = state_directory.join("not-present");
    let state = AppState::with_registry_path(&registry_path).unwrap();
    assert_eq!(
        register_volume_via_api(
            state.clone(),
            local_volume(
                "probe-unavailable",
                &unavailable_source,
                VolumeHealth::Online,
                SentinelStatus::Verified,
            ),
        )
        .await,
        StatusCode::CREATED
    );

    let unavailable_response = build_router(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/Cinevault/Volumes/probe-unavailable/Probe")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unavailable_response.status(), StatusCode::OK);
    let bytes = to_bytes(unavailable_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let unavailable: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(unavailable["status"], "unavailable");
    assert!(unavailable["identity"].is_null());

    let unknown_response = build_router(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/Cinevault/Volumes/not-registered/Probe")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unknown_response.status(), StatusCode::NOT_FOUND);

    fs::remove_dir_all(state_directory).unwrap();
}
