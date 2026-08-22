use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr, TcpStream, UdpSocket};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CastingDeviceType {
    Chromecast,
    Airplay,
    Smartview,
    Dlna,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CastingConnectionState {
    Available,
    Connecting,
    Connected,
    Disconnecting,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CastingDevice {
    pub id: String,
    pub name: String,
    pub address: Option<String>,
    pub port: Option<u16>,
    #[serde(rename = "type")]
    pub device_type: CastingDeviceType,
    pub connected: bool,
    pub state: Option<CastingConnectionState>,
    pub model: Option<String>,
    pub last_seen: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CastingSession {
    pub device: CastingDevice,
    pub media_url: String,
    pub title: Option<String>,
    pub content_type: Option<String>,
    pub current_time: Option<f64>,
    pub duration: Option<f64>,
    pub volume: Option<f64>,
    pub paused: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackPatch {
    pub current_time: Option<f64>,
    pub volume: Option<f64>,
    pub paused: Option<bool>,
}

static ACTIVE_DEVICE: OnceLock<Mutex<Option<CastingDevice>>> = OnceLock::new();
static ACTIVE_SESSION: OnceLock<Mutex<Option<CastingSession>>> = OnceLock::new();

fn active_device() -> &'static Mutex<Option<CastingDevice>> {
    ACTIVE_DEVICE.get_or_init(|| Mutex::new(None))
}

fn active_session() -> &'static Mutex<Option<CastingSession>> {
    ACTIVE_SESSION.get_or_init(|| Mutex::new(None))
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn device_id(kind: &str, address: &str, port: u16) -> String {
    format!("{kind}:{address}:{port}")
}

fn parse_ssdp_headers(packet: &str) -> HashMap<String, String> {
    packet
        .lines()
        .filter_map(|line| line.split_once(':'))
        .map(|(key, value)| (key.trim().to_ascii_lowercase(), value.trim().to_string()))
        .collect()
}

fn host_from_location(location: &str) -> Option<(String, u16)> {
    let remainder = location
        .strip_prefix("http://")
        .or_else(|| location.strip_prefix("https://"))?;
    let authority = remainder.split('/').next()?;
    if let Some((host, port)) = authority.rsplit_once(':') {
        Some((
            host.trim_matches(['[', ']']).to_string(),
            port.parse().ok()?,
        ))
    } else {
        Some((
            authority.to_string(),
            if location.starts_with("https://") {
                443
            } else {
                80
            },
        ))
    }
}

fn classify_ssdp(headers: &HashMap<String, String>) -> CastingDeviceType {
    let haystack = format!(
        "{} {} {}",
        headers.get("server").cloned().unwrap_or_default(),
        headers.get("st").cloned().unwrap_or_default(),
        headers.get("usn").cloned().unwrap_or_default()
    )
    .to_ascii_lowercase();

    if haystack.contains("samsung") {
        CastingDeviceType::Smartview
    } else {
        CastingDeviceType::Dlna
    }
}

fn discover_ssdp() -> Vec<CastingDevice> {
    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(socket) => socket,
        Err(_) => return Vec::new(),
    };
    let _ = socket.set_read_timeout(Some(Duration::from_millis(450)));
    let request = concat!(
        "M-SEARCH * HTTP/1.1\r\n",
        "HOST: 239.255.255.250:1900\r\n",
        "MAN: \"ssdp:discover\"\r\n",
        "MX: 1\r\n",
        "ST: urn:schemas-upnp-org:device:MediaRenderer:1\r\n\r\n"
    );
    let _ = socket.send_to(request.as_bytes(), "239.255.255.250:1900");

    let started = Instant::now();
    let mut devices = HashMap::new();
    while started.elapsed() < Duration::from_millis(1200) {
        let mut buffer = [0_u8; 8192];
        let Ok((size, source)) = socket.recv_from(&mut buffer) else {
            continue;
        };
        let packet = String::from_utf8_lossy(&buffer[..size]);
        let headers = parse_ssdp_headers(&packet);
        let location = headers.get("location").cloned().unwrap_or_default();
        let (host, port) = host_from_location(&location)
            .unwrap_or_else(|| (source.ip().to_string(), source.port()));
        let kind = classify_ssdp(&headers);
        let kind_name = match kind {
            CastingDeviceType::Smartview => "smartview",
            _ => "dlna",
        };
        let name = headers
            .get("server")
            .cloned()
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| format!("Media Renderer {}", source.ip()));
        let id = device_id(kind_name, &host, port);
        devices.insert(
            id.clone(),
            CastingDevice {
                id,
                name,
                address: Some(host),
                port: Some(port),
                device_type: kind,
                connected: false,
                state: Some(CastingConnectionState::Available),
                model: headers.get("server").cloned(),
                last_seen: Some(now_iso()),
            },
        );
    }
    devices.into_values().collect()
}

fn encode_dns_name(name: &str) -> Vec<u8> {
    let mut encoded = Vec::new();
    for label in name.trim_end_matches('.').split('.') {
        encoded.push(label.len() as u8);
        encoded.extend_from_slice(label.as_bytes());
    }
    encoded.push(0);
    encoded
}

fn mdns_query(service: &str) -> Vec<CastingDevice> {
    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(socket) => socket,
        Err(_) => return Vec::new(),
    };
    let _ = socket.set_read_timeout(Some(Duration::from_millis(350)));
    let mut query = vec![0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0];
    query.extend_from_slice(&encode_dns_name(service));
    query.extend_from_slice(&[0, 12, 0, 1]);
    let _ = socket.send_to(&query, "224.0.0.251:5353");

    let started = Instant::now();
    let mut by_address = HashMap::new();
    while started.elapsed() < Duration::from_millis(900) {
        let mut buffer = [0_u8; 9000];
        let Ok((_size, source)) = socket.recv_from(&mut buffer) else {
            continue;
        };
        let (kind, port) = if service.contains("googlecast") {
            (CastingDeviceType::Chromecast, 8009)
        } else {
            (CastingDeviceType::Airplay, 7000)
        };
        let kind_name = if service.contains("googlecast") {
            "chromecast"
        } else {
            "airplay"
        };
        let address = source.ip().to_string();
        let id = device_id(kind_name, &address, port);
        by_address.insert(
            id.clone(),
            CastingDevice {
                id,
                name: format!(
                    "{} {}",
                    if service.contains("googlecast") {
                        "Chromecast"
                    } else {
                        "AirPlay"
                    },
                    address
                ),
                address: Some(address),
                port: Some(port),
                device_type: kind,
                connected: false,
                state: Some(CastingConnectionState::Available),
                model: None,
                last_seen: Some(now_iso()),
            },
        );
    }
    by_address.into_values().collect()
}

fn reachable(device: &CastingDevice) -> bool {
    let Some(address) = device.address.as_deref() else {
        return false;
    };
    let port = device.port.unwrap_or(match device.device_type {
        CastingDeviceType::Chromecast => 8009,
        CastingDeviceType::Airplay => 7000,
        CastingDeviceType::Smartview | CastingDeviceType::Dlna => 80,
    });
    let Ok(ip) = address.parse::<IpAddr>() else {
        return false;
    };
    TcpStream::connect_timeout(&SocketAddr::new(ip, port), Duration::from_secs(2)).is_ok()
}

#[tauri::command]
pub async fn discover_casting_devices() -> Result<Vec<CastingDevice>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let mut devices = discover_ssdp();
        devices.extend(mdns_query("_googlecast._tcp.local."));
        devices.extend(mdns_query("_airplay._tcp.local."));
        let mut unique = HashMap::new();
        for device in devices {
            unique.insert(device.id.clone(), device);
        }
        let mut values: Vec<_> = unique.into_values().collect();
        values.sort_by(|left, right| left.name.cmp(&right.name));
        values
    })
    .await
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn connect_casting_device(mut device: CastingDevice) -> Result<CastingDevice, String> {
    let check = device.clone();
    let is_reachable = tauri::async_runtime::spawn_blocking(move || reachable(&check))
        .await
        .map_err(|error| error.to_string())?;
    if !is_reachable {
        device.state = Some(CastingConnectionState::Error);
        return Err(format!(
            "{} is not reachable on the local network",
            device.name
        ));
    }
    device.connected = true;
    device.state = Some(CastingConnectionState::Connected);
    device.last_seen = Some(now_iso());
    *active_device().lock().map_err(|error| error.to_string())? = Some(device.clone());
    Ok(device)
}

#[tauri::command]
pub async fn disconnect_casting_device(device: CastingDevice) -> Result<CastingDevice, String> {
    let mut disconnected = device;
    disconnected.connected = false;
    disconnected.state = Some(CastingConnectionState::Available);
    *active_device().lock().map_err(|error| error.to_string())? = None;
    *active_session().lock().map_err(|error| error.to_string())? = None;
    Ok(disconnected)
}

async fn start_airplay(session: &CastingSession) -> Result<(), String> {
    let host = session
        .device
        .address
        .as_deref()
        .ok_or("AirPlay device has no address")?;
    let port = session.device.port.unwrap_or(7000);
    let endpoint = format!("http://{host}:{port}/play");
    let body = format!(
        "Content-Location: {}\nStart-Position: {}\n",
        session.media_url,
        session.current_time.unwrap_or(0.0)
    );
    reqwest::Client::new()
        .post(endpoint)
        .header("Content-Type", "text/parameters")
        .body(body)
        .send()
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn start_casting(session: CastingSession) -> Result<String, String> {
    if session.media_url.trim().is_empty() {
        return Err("Select a reachable media URL before starting playback".to_string());
    }
    match session.device.device_type {
        CastingDeviceType::Airplay => start_airplay(&session).await?,
        CastingDeviceType::Chromecast => {
            return Err("Chromecast transport requires the bundled Cast bridge service".to_string())
        }
        CastingDeviceType::Smartview | CastingDeviceType::Dlna => {
            return Err(
                "DLNA playback requires renderer AVTransport metadata from discovery".to_string(),
            )
        }
    }
    *active_session().lock().map_err(|error| error.to_string())? = Some(session.clone());
    Ok(format!("Casting started on {}", session.device.name))
}

#[tauri::command]
pub async fn update_casting_playback(patch: PlaybackPatch) -> Result<CastingSession, String> {
    let mut guard = active_session().lock().map_err(|error| error.to_string())?;
    let session = guard.as_mut().ok_or("No active casting session")?;
    if let Some(value) = patch.current_time {
        session.current_time = Some(value.max(0.0));
    }
    if let Some(value) = patch.volume {
        session.volume = Some(value.clamp(0.0, 1.0));
    }
    if let Some(value) = patch.paused {
        session.paused = Some(value);
    }
    Ok(session.clone())
}
