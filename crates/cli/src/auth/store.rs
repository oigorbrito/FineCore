//! Atomic, owner-restricted storage for FineCore-owned OAuth credentials.
//!
//! Secure ACL/mode is applied to the parent and temporary file *before*
//! any token data is written. No external Codex CLI files are managed here.
use std::{
    fs,
    io::{self, Write},
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

fn regular_target_or_absent(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => Ok(()),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "OAuth credential path must be a regular file, not a link or directory",
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn prepare_parent(parent: &Path) -> io::Result<()> {
    fs::create_dir_all(parent)?;
    let metadata = fs::symlink_metadata(parent)?;
    if !metadata.file_type().is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "OAuth credential parent must be a real directory",
        ));
    }
    restrict(parent, true)
}

pub fn write_private_json(path: &Path, value: &serde_json::Value) -> Result<(), String> {
    if !path.is_absolute() {
        return Err("credential path must be absolute".into());
    }
    let parent = path.parent().ok_or("invalid credential path")?;
    prepare_parent(parent).map_err(|error| format!("cannot secure OAuth directory: {error}"))?;
    regular_target_or_absent(path).map_err(|error| error.to_string())?;
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|_| "cannot encode OAuth credential JSON".to_string())?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temp = parent.join(format!(".oauth-{}-{nonce}.tmp", std::process::id()));
    let result = (|| -> io::Result<()> {
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temp)?;
        // On Windows an inherited ACL is not sufficient: explicitly protect
        // the empty temp file before it contains access/refresh tokens.
        restrict(&temp, false)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        // Never replace a symlink or directory installed during the write.
        regular_target_or_absent(path)?;
        fs::rename(&temp, path)?;
        // The rename preserves temp security attributes; check again so
        // failure is surfaced instead of silently accepting open permissions.
        restrict(path, false)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result.map_err(|error| format!("OAuth credential write failed: {error}"))
}

#[cfg(unix)]
fn restrict(path: &Path, directory: bool) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(
        path,
        fs::Permissions::from_mode(if directory { 0o700 } else { 0o600 }),
    )
}

#[cfg(windows)]
fn restrict(path: &Path, _directory: bool) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Authorization::{
        ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
    };
    use windows_sys::Win32::Security::{
        SetFileSecurityW, DACL_SECURITY_INFORMATION, PROTECTED_DACL_SECURITY_INFORMATION,
    };

    // No inherited ACEs. Full control belongs only to file owner and SYSTEM.
    let sddl: Vec<u16> = "D:P(A;;FA;;;OW)(A;;FA;;;SY)\0".encode_utf16().collect();
    let mut descriptor = std::ptr::null_mut();
    let mut descriptor_size = 0;
    // SAFETY: SDDL is UTF-16 NUL terminated and Windows owns descriptor.
    let converted = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            SDDL_REVISION_1,
            &mut descriptor,
            &mut descriptor_size,
        )
    };
    if converted == 0 {
        return Err(io::Error::last_os_error());
    }
    let mut wide_path: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide_path.push(0);
    // SAFETY: path and descriptor are valid for the duration of this call.
    let secured = unsafe {
        SetFileSecurityW(
            wide_path.as_ptr(),
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            descriptor,
        )
    };
    let error = (secured == 0).then(io::Error::last_os_error);
    // SAFETY: the Security API returned a LocalAlloc descriptor.
    unsafe { LocalFree(descriptor) };
    error.map_or(Ok(()), Err)
}

#[cfg(not(any(unix, windows)))]
fn restrict(_path: &Path, _directory: bool) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "OAuth secure storage unsupported on this platform",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn directory() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "finecore-oauth-permissions-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn atomic_write_replaces_tokens_and_preserves_json() {
        let dir = directory();
        let path = dir.join("oauth.json");
        write_private_json(&path, &serde_json::json!({"access_token":"first"})).unwrap();
        write_private_json(&path, &serde_json::json!({"access_token":"second"})).unwrap();
        let saved: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(saved["access_token"], "second");
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn rejects_links_without_replacing_target() {
        let dir = directory();
        let target = dir.join("target.txt");
        fs::write(&target, "must not change").unwrap();
        let link = dir.join("oauth.json");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&target, &link).unwrap();
            assert!(write_private_json(&link, &serde_json::json!({"access_token":"secret"}))
                .is_err());
            assert_eq!(fs::read_to_string(&target).unwrap(), "must not change");
        }
        #[cfg(windows)]
        {
            if std::os::windows::fs::symlink_file(&target, &link).is_ok() {
                assert!(write_private_json(&link, &serde_json::json!({"access_token":"secret"}))
                    .is_err());
                assert_eq!(fs::read_to_string(&target).unwrap(), "must not change");
            }
        }
        fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn file_and_directory_are_owner_only_even_after_replacement() {
        use std::os::unix::fs::PermissionsExt;
        let dir = directory();
        let path = dir.join("oauth.json");
        write_private_json(&path, &serde_json::json!({"refresh_token":"one"})).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o666)).unwrap();
        write_private_json(&path, &serde_json::json!({"refresh_token":"two"})).unwrap();
        assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
        assert_eq!(fs::metadata(&dir).unwrap().permissions().mode() & 0o777, 0o700);
        fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn file_and_temp_are_owner_system_only_across_rotation() {
        let dir = directory();
        let path = dir.join("oauth.json");
        write_private_json(&path, &serde_json::json!({"refresh_token":"one"})).unwrap();
        write_private_json(&path, &serde_json::json!({"refresh_token":"two"})).unwrap();
        let output = std::process::Command::new("icacls")
            .arg(&path)
            .output()
            .expect("icacls required by Windows test");
        assert!(output.status.success());
        let acl = String::from_utf8_lossy(&output.stdout);
        assert_eq!(acl.matches(":(F)").count(), 2, "unexpected OAuth DACL: {acl}");
        let parent_acl = std::process::Command::new("icacls").arg(&dir).output().unwrap();
        assert!(parent_acl.status.success());
        assert_eq!(String::from_utf8_lossy(&parent_acl.stdout).matches(":(F)").count(), 2);
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn relative_credential_path_fails() {
        assert!(write_private_json(
            Path::new("workspace-token.json"),
            &serde_json::json!({})
        ).is_err());
    }
}
