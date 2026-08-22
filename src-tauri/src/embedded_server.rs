use crate::build_identity;
use crate::db::{Database, MediaItem, RemoteAccessPrincipal};
use crate::metadata_provider_config;
use crate::shared_contracts::{
    validate_metadata_provider_contract, MetadataProviderRegistryContract,
    MetadataProviderRegistryInterface,
};
use axum::body::Body;
use axum::extract::{ConnectInfo, DefaultBodyLimit, Path, State};
use axum::http::{header, HeaderMap, HeaderValue, Response, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Path as FilePath, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio::sync::oneshot;
use tokio_util::io::ReaderStream;

const DEFAULT_PORT: u16 = 32400;
const MAX_ARTWORK_BYTES: usize = 25 * 1024 * 1024;
const MAX_LOGIN_FAILURES: u32 = 5;
const LOGIN_BLOCK_DURATION: Duration = Duration::from_secs(5 * 60);
const LOGIN_ATTEMPT_RETENTION: Duration = Duration::from_secs(60 * 60);
// This domain is intentionally stable so existing opaque remote media keys do not change on upgrade.
const REMOTE_MEDIA_KEY_DOMAIN: &[u8] = b"cinavault-build-170-remote-media-v1";

#[derive(Clone)]
struct HttpState {
    database_path: String,
    login_attempts: Arc<Mutex<HashMap<std::net::IpAddr, LoginAttempt>>>,
}

#[derive(Clone, Copy)]
struct LoginAttempt {
    failures: u32,
    blocked_until: Option<std::time::Instant>,
    last_seen: std::time::Instant,
}

struct ServerRuntime {
    port: u16,
    shutdown: oneshot::Sender<()>,
}

static DATABASE_PATH: OnceLock<String> = OnceLock::new();
static SERVER_RUNTIME: OnceLock<Mutex<Option<ServerRuntime>>> = OnceLock::new();

fn runtime() -> &'static Mutex<Option<ServerRuntime>> {
    SERVER_RUNTIME.get_or_init(|| Mutex::new(None))
}

pub fn configure(database_path: String) {
    let _ = DATABASE_PATH.set(database_path);
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PasswordLogin {
    email: String,
    password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccessKeyLogin {
    access_key: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ServerStatus {
    running: bool,
    port: u16,
    local_url: String,
    remote_ready: bool,
    authentication: &'static str,
    remote_transport: &'static str,
    local_paths_exposed: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ServerInfo {
    name: String,
    product: &'static str,
    version: String,
    build: String,
    display_name: String,
    release_tag: String,
    account_email: String,
    permissions: Vec<String>,
    remote_transport: &'static str,
    media_identifiers: &'static str,
    local_paths_exposed: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LibraryCount {
    total_items: i64,
    count_policy: &'static str,
    capped: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RemoteMediaItem {
    media_key: String,
    title: String,
    media_type: String,
    year: Option<i32>,
    rating: Option<f64>,
    overview: Option<String>,
    genre: Option<String>,
    duration: Option<i64>,
    file_size: Option<i64>,
    resolution: Option<String>,
    codec: Option<String>,
    verified: bool,
    watched: bool,
    favorite: bool,
    date_added: String,
    last_played: Option<String>,
    tmdb_id: Option<String>,
    imdb_id: Option<String>,
    artwork_url: Option<String>,
    stream_url: String,
}

fn open_database(path: &str) -> Result<Database, (StatusCode, String)> {
    Database::new(path).map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database unavailable: {error}"),
        )
    })
}

fn media_key(item: &MediaItem) -> Option<String> {
    let id = item.id?;
    let mut hasher = Sha256::new();
    hasher.update(REMOTE_MEDIA_KEY_DOMAIN);
    hasher.update(id.to_le_bytes());
    hasher.update(item.file_path.as_bytes());
    Some(
        hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    )
}

fn find_item_by_key(database: &Database, key: &str) -> Result<Option<MediaItem>, String> {
    database
        .get_media_items_data(None, None, None)
        .map_err(|error| error.to_string())
        .map(|items| {
            items
                .into_iter()
                .find(|item| media_key(item).as_deref() == Some(key))
        })
}

fn preferred_artwork(item: &MediaItem) -> Option<(&'static str, String)> {
    item.poster_path
        .as_ref()
        .filter(|value| !value.trim().is_empty())
        .map(|value| ("poster", value.clone()))
        .or_else(|| {
            item.backdrop_path
                .as_ref()
                .filter(|value| !value.trim().is_empty())
                .map(|value| ("backdrop", value.clone()))
        })
}

fn remote_media_item(item: MediaItem) -> Option<RemoteMediaItem> {
    let key = media_key(&item)?;
    let artwork_url =
        preferred_artwork(&item).map(|(kind, _)| format!("/api/artwork/{key}/{kind}"));
    Some(RemoteMediaItem {
        media_key: key.clone(),
        title: item.title,
        media_type: item.media_type,
        year: item.year,
        rating: item.rating,
        overview: item.overview,
        genre: item.genre,
        duration: item.duration,
        file_size: item.file_size,
        resolution: item.resolution,
        codec: item.codec,
        verified: item.verified,
        watched: item.watched,
        favorite: item.favorite,
        date_added: item.date_added,
        last_played: item.last_played,
        tmdb_id: item.tmdb_id,
        imdb_id: item.imdb_id,
        artwork_url,
        stream_url: format!("/api/stream/{key}"),
    })
}

fn register_session(principal: RemoteAccessPrincipal) -> Json<RemoteAccessPrincipal> {
    Json(principal)
}

fn login_is_allowed(state: &HttpState, client: std::net::IpAddr) -> bool {
    let now = std::time::Instant::now();
    let Ok(mut attempts) = state.login_attempts.lock() else {
        return true;
    };
    attempts.retain(|_, attempt| {
        now.duration_since(attempt.last_seen) < LOGIN_ATTEMPT_RETENTION
            || attempt.blocked_until.is_some_and(|until| until > now)
    });
    let attempt = attempts.entry(client).or_insert(LoginAttempt {
        failures: 0,
        blocked_until: None,
        last_seen: now,
    });
    attempt.last_seen = now;
    !attempt.blocked_until.is_some_and(|until| until > now)
}

fn record_login_failure(state: &HttpState, client: std::net::IpAddr) {
    let now = std::time::Instant::now();
    if let Ok(mut attempts) = state.login_attempts.lock() {
        let attempt = attempts.entry(client).or_insert(LoginAttempt {
            failures: 0,
            blocked_until: None,
            last_seen: now,
        });
        attempt.last_seen = now;
        attempt.failures = attempt.failures.saturating_add(1);
        if attempt.failures >= MAX_LOGIN_FAILURES {
            attempt.failures = 0;
            attempt.blocked_until = Some(now + LOGIN_BLOCK_DURATION);
        }
    }
}

fn record_login_success(state: &HttpState, client: std::net::IpAddr) {
    if let Ok(mut attempts) = state.login_attempts.lock() {
        attempts.remove(&client);
    }
}

async fn login_password(
    State(state): State<Arc<HttpState>>,
    ConnectInfo(client): ConnectInfo<SocketAddr>,
    Json(payload): Json<PasswordLogin>,
) -> Result<Json<RemoteAccessPrincipal>, (StatusCode, String)> {
    if !login_is_allowed(&state, client.ip()) {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            "Too many failed login attempts. Try again later.".into(),
        ));
    }
    let database = open_database(&state.database_path)?;
    match database
        .authenticate_remote_password(&payload.email, &payload.password)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error))?
    {
        Some(principal) => {
            record_login_success(&state, client.ip());
            Ok(register_session(principal))
        }
        None => {
            record_login_failure(&state, client.ip());
            Err((
                StatusCode::UNAUTHORIZED,
                "Invalid account credentials".into(),
            ))
        }
    }
}

async fn login_access_key(
    State(state): State<Arc<HttpState>>,
    ConnectInfo(client): ConnectInfo<SocketAddr>,
    Json(payload): Json<AccessKeyLogin>,
) -> Result<Json<RemoteAccessPrincipal>, (StatusCode, String)> {
    if !login_is_allowed(&state, client.ip()) {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            "Too many failed login attempts. Try again later.".into(),
        ));
    }
    let database = open_database(&state.database_path)?;
    match database
        .authenticate_remote_access_key(&payload.access_key)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error))?
    {
        Some(principal) => {
            record_login_success(&state, client.ip());
            Ok(register_session(principal))
        }
        None => {
            record_login_failure(&state, client.ip());
            Err((
                StatusCode::UNAUTHORIZED,
                "Invalid account access key".into(),
            ))
        }
    }
}

async fn authenticated_principal(
    state: &HttpState,
    headers: &HeaderMap,
    permission: &str,
) -> Result<RemoteAccessPrincipal, (StatusCode, String)> {
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or((StatusCode::UNAUTHORIZED, "Bearer token required".into()))?;

    let database = open_database(&state.database_path)?;
    let principal = database
        .validate_remote_access_session(token)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error))?
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "Session is invalid, revoked, or expired".into(),
        ))?;

    if !principal
        .permissions
        .iter()
        .any(|value| value == permission)
    {
        return Err((
            StatusCode::FORBIDDEN,
            "Account lacks required permission".into(),
        ));
    }
    Ok(principal)
}

fn hardened_response_headers(response: &mut Response<Body>) {
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-store, max-age=0"),
    );
    response
        .headers_mut()
        .insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    response.headers_mut().insert(
        header::HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    response.headers_mut().insert(
        header::HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("no-referrer"),
    );
}

async fn health() -> impl IntoResponse {
    let build = build_identity::current();
    Json(serde_json::json!({
        "status": "ok",
        "product": "CinaVault Embedded Media Server",
        "version": build.semantic_version,
        "build": build.display_build,
        "displayName": build.display_name,
        "releaseTag": build.release_tag,
        "remoteTransport": "HTTPS relay required by default",
        "localPathsExposed": false
    }))
}

async fn server_info(
    State(state): State<Arc<HttpState>>,
    headers: HeaderMap,
) -> Result<Json<ServerInfo>, (StatusCode, String)> {
    let principal = authenticated_principal(&state, &headers, "server:read").await?;
    let build = build_identity::current();
    Ok(Json(ServerInfo {
        name: build.product_name.clone(),
        product: "CinaVault Embedded Media Server",
        version: build.semantic_version.clone(),
        build: build.display_build.clone(),
        display_name: build.display_name.clone(),
        release_tag: build.release_tag.clone(),
        account_email: principal.email,
        permissions: principal.permissions,
        remote_transport: "HTTPS relay",
        media_identifiers: "opaque SHA-256 media keys",
        local_paths_exposed: false,
    }))
}

async fn metadata_providers(
    State(state): State<Arc<HttpState>>,
    headers: HeaderMap,
) -> Result<Json<MetadataProviderRegistryContract>, (StatusCode, String)> {
    authenticated_principal(&state, &headers, "server:read").await?;
    let registry = metadata_provider_config::public_registry()
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error))?;
    let contract = registry.metadata_provider_contract();
    validate_metadata_provider_contract(&contract)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error))?;
    Ok(Json(contract))
}

async fn library_count(
    State(state): State<Arc<HttpState>>,
    headers: HeaderMap,
) -> Result<Json<LibraryCount>, (StatusCode, String)> {
    authenticated_principal(&state, &headers, "library:read").await?;
    let database = open_database(&state.database_path)?;
    let total_items = database
        .conn
        .query_row(
            "SELECT COUNT(*) FROM media_items WHERE media_type <> 'photo'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    Ok(Json(LibraryCount {
        total_items,
        count_policy: "all indexed non-artwork media rows",
        capped: false,
    }))
}

async fn library(
    State(state): State<Arc<HttpState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<RemoteMediaItem>>, (StatusCode, String)> {
    authenticated_principal(&state, &headers, "library:read").await?;
    let database = open_database(&state.database_path)?;
    let items = database
        .get_media_items_data(None, None, None)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    Ok(Json(
        items.into_iter().filter_map(remote_media_item).collect(),
    ))
}

async fn library_item(
    State(state): State<Arc<HttpState>>,
    headers: HeaderMap,
    Path(media_key): Path<String>,
) -> Result<Json<RemoteMediaItem>, (StatusCode, String)> {
    authenticated_principal(&state, &headers, "library:read").await?;
    let database = open_database(&state.database_path)?;
    find_item_by_key(&database, &media_key)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error))?
        .and_then(remote_media_item)
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Media item not found".into()))
}

fn content_type(path: &FilePath) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "mp4" | "m4v" => "video/mp4",
        "mkv" => "video/x-matroska",
        "webm" => "video/webm",
        "avi" => "video/x-msvideo",
        "mp3" => "audio/mpeg",
        "m4a" => "audio/mp4",
        "flac" => "audio/flac",
        "wav" => "audio/wav",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => "application/octet-stream",
    }
}

fn requested_range(headers: &HeaderMap, size: u64) -> Option<(u64, u64)> {
    let value = headers.get(header::RANGE)?.to_str().ok()?;
    let range = value.strip_prefix("bytes=")?.split(',').next()?;
    let (start, end) = range.split_once('-')?;
    let start = start.parse::<u64>().ok()?;
    let end = if end.trim().is_empty() {
        size.saturating_sub(1)
    } else {
        end.parse::<u64>().ok()?.min(size.saturating_sub(1))
    };
    (start <= end && end < size).then_some((start, end))
}

fn selected_artwork(item: &MediaItem, requested_kind: Option<&str>) -> Option<(String, String)> {
    match requested_kind {
        Some("poster") => item
            .poster_path
            .clone()
            .filter(|value| !value.trim().is_empty())
            .map(|value| (value, "poster".to_string())),
        Some("backdrop") => item
            .backdrop_path
            .clone()
            .filter(|value| !value.trim().is_empty())
            .map(|value| (value, "backdrop".to_string())),
        Some(_) => None,
        None => preferred_artwork(item).map(|(kind, value)| (value, kind.to_string())),
    }
}

async fn read_artwork_bytes(artwork: &str) -> Result<(Vec<u8>, String), (StatusCode, String)> {
    let (bytes, mime) = if artwork.starts_with("https://") {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::limited(3))
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
        let response = client
            .get(artwork)
            .send()
            .await
            .map_err(|error| (StatusCode::BAD_GATEWAY, error.to_string()))?
            .error_for_status()
            .map_err(|error| (StatusCode::BAD_GATEWAY, error.to_string()))?;
        if response
            .content_length()
            .is_some_and(|length| length > MAX_ARTWORK_BYTES as u64)
        {
            return Err((
                StatusCode::PAYLOAD_TOO_LARGE,
                "Artwork exceeds 25 MiB".into(),
            ));
        }
        let mime = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("application/octet-stream")
            .split(';')
            .next()
            .unwrap_or("application/octet-stream")
            .trim()
            .to_string();
        let bytes = response
            .bytes()
            .await
            .map_err(|error| (StatusCode::BAD_GATEWAY, error.to_string()))?;
        (bytes.to_vec(), mime)
    } else {
        let path = PathBuf::from(artwork);
        let metadata = tokio::fs::metadata(&path)
            .await
            .map_err(|error| (StatusCode::NOT_FOUND, error.to_string()))?;
        if metadata.len() > MAX_ARTWORK_BYTES as u64 {
            return Err((
                StatusCode::PAYLOAD_TOO_LARGE,
                "Artwork exceeds 25 MiB".into(),
            ));
        }
        let bytes = tokio::fs::read(&path)
            .await
            .map_err(|error| (StatusCode::NOT_FOUND, error.to_string()))?;
        (bytes, content_type(&path).to_string())
    };

    if bytes.is_empty() {
        return Err((StatusCode::NOT_FOUND, "Artwork is empty".into()));
    }
    if bytes.len() > MAX_ARTWORK_BYTES {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            "Artwork exceeds 25 MiB".into(),
        ));
    }
    if !mime.starts_with("image/") {
        return Err((
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Artwork response is not an image".into(),
        ));
    }
    Ok((bytes, mime))
}

async fn artwork_response(
    state: Arc<HttpState>,
    headers: HeaderMap,
    media_key: String,
    requested_kind: Option<String>,
) -> Result<Response<Body>, (StatusCode, String)> {
    authenticated_principal(&state, &headers, "library:read").await?;
    let database = open_database(&state.database_path)?;
    let item = find_item_by_key(&database, &media_key)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error))?
        .ok_or((StatusCode::NOT_FOUND, "Media item not found".into()))?;
    let (artwork, _) = selected_artwork(&item, requested_kind.as_deref())
        .ok_or((StatusCode::NOT_FOUND, "Artwork not available".into()))?;
    let (bytes, mime) = read_artwork_bytes(&artwork).await?;

    let mut response = Response::new(Body::from(bytes));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&mime)
            .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?,
    );
    hardened_response_headers(&mut response);
    Ok(response)
}

async fn artwork_media(
    State(state): State<Arc<HttpState>>,
    headers: HeaderMap,
    Path(media_key): Path<String>,
) -> Result<Response<Body>, (StatusCode, String)> {
    artwork_response(state, headers, media_key, None).await
}

async fn artwork_media_kind(
    State(state): State<Arc<HttpState>>,
    headers: HeaderMap,
    Path((media_key, kind)): Path<(String, String)>,
) -> Result<Response<Body>, (StatusCode, String)> {
    artwork_response(state, headers, media_key, Some(kind)).await
}

async fn stream_media(
    State(state): State<Arc<HttpState>>,
    headers: HeaderMap,
    Path(media_key): Path<String>,
) -> Result<Response<Body>, (StatusCode, String)> {
    authenticated_principal(&state, &headers, "stream:play").await?;
    let database = open_database(&state.database_path)?;
    let item = find_item_by_key(&database, &media_key)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error))?
        .ok_or((StatusCode::NOT_FOUND, "Media item not found".into()))?;

    let path = PathBuf::from(item.file_path);
    let mut file = tokio::fs::File::open(&path)
        .await
        .map_err(|error| (StatusCode::NOT_FOUND, error.to_string()))?;
    let size = file
        .metadata()
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?
        .len();

    if size == 0 {
        return Err((
            StatusCode::RANGE_NOT_SATISFIABLE,
            "Media file is empty".into(),
        ));
    }

    let (status, start, end) = requested_range(&headers, size)
        .map(|(start, end)| (StatusCode::PARTIAL_CONTENT, start, end))
        .unwrap_or((StatusCode::OK, 0, size.saturating_sub(1)));
    let length = end.saturating_sub(start).saturating_add(1);
    file.seek(std::io::SeekFrom::Start(start))
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    let stream = ReaderStream::new(file.take(length));
    let mut response = Response::new(Body::from_stream(stream));
    *response.status_mut() = status;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(content_type(&path)),
    );
    response.headers_mut().insert(
        header::CONTENT_LENGTH,
        HeaderValue::from_str(&length.to_string())
            .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?,
    );
    response
        .headers_mut()
        .insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    if status == StatusCode::PARTIAL_CONTENT {
        response.headers_mut().insert(
            header::CONTENT_RANGE,
            HeaderValue::from_str(&format!("bytes {start}-{end}/{size}"))
                .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?,
        );
    }
    hardened_response_headers(&mut response);
    Ok(response)
}

fn router(state: Arc<HttpState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/auth/password", post(login_password))
        .route("/api/auth/access-key", post(login_access_key))
        .route("/api/server/info", get(server_info))
        .route("/api/metadata/providers", get(metadata_providers))
        .route("/api/library", get(library))
        .route("/api/library/count", get(library_count))
        .route("/api/library/{media_key}", get(library_item))
        .route("/api/artwork/{media_key}", get(artwork_media))
        .route("/api/artwork/{media_key}/{kind}", get(artwork_media_kind))
        .route("/api/stream/{media_key}", get(stream_media))
        // Native clients and the authenticated relay do not require browser cross-origin access.
        .layer(DefaultBodyLimit::max(16 * 1024))
        .with_state(state)
}

#[tauri::command]
pub async fn start_embedded_server(port: Option<u16>) -> Result<serde_json::Value, String> {
    let port = port.unwrap_or(DEFAULT_PORT);
    {
        let guard = runtime().lock().map_err(|error| error.to_string())?;
        if let Some(active) = guard.as_ref() {
            return Ok(serde_json::json!({
                "running": true,
                "port": active.port,
                "localUrl": format!("http://127.0.0.1:{}", active.port),
                "remoteReady": false,
                "authentication": "CinaVault account session",
                "remoteTransport": "HTTPS relay required by default",
                "localPathsExposed": false
            }));
        }
    }

    let database_path = DATABASE_PATH
        .get()
        .cloned()
        .ok_or("Embedded server database is not configured")?;
    // The public relay terminates TLS separately; the local media server is never exposed on the LAN.
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .map_err(|error| format!("Unable to bind embedded server on port {port}: {error}"))?;
    let state = Arc::new(HttpState {
        database_path,
        login_attempts: Arc::new(Mutex::new(HashMap::new())),
    });
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    *runtime().lock().map_err(|error| error.to_string())? = Some(ServerRuntime {
        port,
        shutdown: shutdown_tx,
    });

    tauri::async_runtime::spawn(async move {
        let result = axum::serve(
            listener,
            router(state).into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(async {
            let _ = shutdown_rx.await;
        })
        .await;
        if let Err(error) = result {
            log::error!("Embedded media server stopped unexpectedly: {error}");
        }
        if let Ok(mut guard) = runtime().lock() {
            *guard = None;
        }
    });

    Ok(serde_json::json!({
        "running": true,
        "port": port,
        "localUrl": format!("http://127.0.0.1:{port}"),
        "remoteReady": false,
        "authentication": "CinaVault account session",
        "remoteTransport": "HTTPS relay required by default",
        "localPathsExposed": false
    }))
}

#[tauri::command]
pub async fn stop_embedded_server() -> Result<serde_json::Value, String> {
    let active = runtime().lock().map_err(|error| error.to_string())?.take();
    if let Some(active) = active {
        let _ = active.shutdown.send(());
    }
    Ok(serde_json::json!({ "running": false }))
}

#[tauri::command]
pub fn get_embedded_server_status() -> Result<serde_json::Value, String> {
    let guard = runtime().lock().map_err(|error| error.to_string())?;
    let status = if let Some(active) = guard.as_ref() {
        ServerStatus {
            running: true,
            port: active.port,
            local_url: format!("http://127.0.0.1:{}", active.port),
            remote_ready: false,
            authentication: "CinaVault account session",
            remote_transport: "HTTPS relay required by default",
            local_paths_exposed: false,
        }
    } else {
        ServerStatus {
            running: false,
            port: DEFAULT_PORT,
            local_url: format!("http://127.0.0.1:{DEFAULT_PORT}"),
            remote_ready: false,
            authentication: "CinaVault account session",
            remote_transport: "HTTPS relay required by default",
            local_paths_exposed: false,
        }
    };
    serde_json::to_value(status).map_err(|error| error.to_string())
}
