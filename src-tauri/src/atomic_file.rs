use std::io::Write;
use std::path::{Path, PathBuf};

fn temporary_path(destination: &Path) -> Result<PathBuf, String> {
    let parent = destination
        .parent()
        .ok_or_else(|| "destination has no parent directory".to_string())?;
    let file_name = destination
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "destination has an invalid filename".to_string())?;
    Ok(parent.join(format!(".{file_name}.{}.tmp", std::process::id())))
}

pub fn write_verified_atomic(destination: &Path, bytes: &[u8]) -> Result<(), String> {
    let temporary = temporary_path(destination)?;
    let result = (|| {
        {
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(&temporary)
                .map_err(|error| format!("atomic create failed: {error}"))?;
            file.write_all(bytes)
                .map_err(|error| format!("atomic write failed: {error}"))?;
            file.flush()
                .map_err(|error| format!("atomic flush failed: {error}"))?;
            file.sync_all()
                .map_err(|error| format!("atomic sync failed: {error}"))?;
        }

        let staged = std::fs::read(&temporary)
            .map_err(|error| format!("atomic verification read failed: {error}"))?;
        if staged != bytes {
            return Err("atomic verification failed: staged bytes differ".to_string());
        }

        replace_file(&temporary, destination)?;

        let persisted = std::fs::read(destination)
            .map_err(|error| format!("atomic persisted read failed: {error}"))?;
        if persisted != bytes {
            return Err("atomic verification failed: persisted bytes differ".to_string());
        }
        Ok(())
    })();

    if temporary.exists() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

#[cfg(target_os = "windows")]
fn replace_file(source: &Path, destination: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;

    const MOVEFILE_REPLACE_EXISTING: u32 = 0x0000_0001;
    const MOVEFILE_WRITE_THROUGH: u32 = 0x0000_0008;

    #[link(name = "Kernel32")]
    extern "system" {
        fn MoveFileExW(existing: *const u16, new_name: *const u16, flags: u32) -> i32;
    }

    let source_wide: Vec<u16> = source
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let destination_wide: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let succeeded = unsafe {
        MoveFileExW(
            source_wide.as_ptr(),
            destination_wide.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if succeeded == 0 {
        Err(format!(
            "atomic replace failed: {}",
            std::io::Error::last_os_error()
        ))
    } else {
        Ok(())
    }
}

#[cfg(not(target_os = "windows"))]
fn replace_file(source: &Path, destination: &Path) -> Result<(), String> {
    std::fs::rename(source, destination).map_err(|error| format!("atomic replace failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::write_verified_atomic;

    #[test]
    fn atomically_creates_and_replaces_file() {
        let root = std::env::temp_dir().join(format!("cinavault-atomic-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let destination = root.join("movie.nfo");
        write_verified_atomic(&destination, b"first").unwrap();
        write_verified_atomic(&destination, b"second").unwrap();
        assert_eq!(std::fs::read(&destination).unwrap(), b"second");
        let _ = std::fs::remove_dir_all(root);
    }
}
