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
    assert_eq!(payload["contract_version"], "v3alpha1");
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
