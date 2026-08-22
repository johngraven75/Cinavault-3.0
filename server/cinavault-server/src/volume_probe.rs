use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VolumeIdentityKind {
    WindowsVolumeGuid,
    PathFingerprint,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct VolumeIdentity {
    pub kind: VolumeIdentityKind,
    pub value: String,
    pub volume_path: Option<String>,
    pub serial_number: Option<String>,
    pub filesystem: Option<String>,
}

pub fn probe_route(path: &Path) -> Result<VolumeIdentity, String> {
    ensure_readable_directory(path)?;
    platform_volume_identity(path)
}

fn ensure_readable_directory(path: &Path) -> Result<(), String> {
    if !path.is_dir() {
        return Err(
            "path does not exist, is not a directory, or the volume is disconnected".to_owned(),
        );
    }

    let mut entries = fs::read_dir(path).map_err(|error| error.to_string())?;
    if let Some(entry) = entries.next() {
        entry.map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[cfg(windows)]
fn platform_volume_identity(path: &Path) -> Result<VolumeIdentity, String> {
    windows_volume_identity(path).or_else(|_| Ok(path_fingerprint(path)))
}

#[cfg(not(windows))]
fn platform_volume_identity(path: &Path) -> Result<VolumeIdentity, String> {
    Ok(path_fingerprint(path))
}

fn path_fingerprint(path: &Path) -> VolumeIdentity {
    let canonical = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    VolumeIdentity {
        kind: VolumeIdentityKind::PathFingerprint,
        value: format!("path:{}", canonical.to_string_lossy()),
        volume_path: None,
        serial_number: None,
        filesystem: None,
    }
}

#[cfg(windows)]
fn windows_volume_identity(path: &Path) -> Result<VolumeIdentity, String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        GetVolumeInformationW, GetVolumeNameForVolumeMountPointW, GetVolumePathNameW,
    };

    const PATH_CAPACITY: usize = 32_768;
    let mut input = path.as_os_str().encode_wide().collect::<Vec<u16>>();
    input.push(0);
    let mut volume_path = vec![0_u16; PATH_CAPACITY];

    let volume_path_length = unsafe {
        GetVolumePathNameW(
            input.as_ptr(),
            volume_path.as_mut_ptr(),
            volume_path.len() as u32,
        )
    };
    if volume_path_length == 0 {
        return Err("Windows did not resolve a volume path for the registered route".to_owned());
    }

    let root = nul_terminated_to_string(&volume_path);
    let mut root_wide = root.encode_utf16().collect::<Vec<u16>>();
    root_wide.push(0);
    let mut volume_guid = vec![0_u16; PATH_CAPACITY];
    let volume_guid_ok = unsafe {
        GetVolumeNameForVolumeMountPointW(
            root_wide.as_ptr(),
            volume_guid.as_mut_ptr(),
            volume_guid.len() as u32,
        )
    };
    if volume_guid_ok == 0 {
        return Err("Windows did not expose a volume GUID for the registered route".to_owned());
    }

    let mut serial_number = 0_u32;
    let mut filesystem = vec![0_u16; PATH_CAPACITY];
    let information_ok = unsafe {
        GetVolumeInformationW(
            root_wide.as_ptr(),
            std::ptr::null_mut(),
            0,
            &mut serial_number,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            filesystem.as_mut_ptr(),
            filesystem.len() as u32,
        )
    };
    if information_ok == 0 {
        return Err(
            "Windows did not return volume information for the registered route".to_owned(),
        );
    }

    Ok(VolumeIdentity {
        kind: VolumeIdentityKind::WindowsVolumeGuid,
        value: nul_terminated_to_string(&volume_guid),
        volume_path: Some(root),
        serial_number: Some(format!("{serial_number:08X}")),
        filesystem: Some(nul_terminated_to_string(&filesystem)),
    })
}

#[cfg(windows)]
fn nul_terminated_to_string(value: &[u16]) -> String {
    let length = value
        .iter()
        .position(|character| *character == 0)
        .unwrap_or(value.len());
    String::from_utf16_lossy(&value[..length])
}
