use super::*;

#[cfg(test)]
thread_local! {
    pub(super) static TEST_FILE: std::cell::RefCell<Option<String>> =
        const { std::cell::RefCell::new(None) };
}

static CONFIG_WRITE_MUTEX: Mutex<()> = Mutex::new(());
static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(in crate::config) type ConfigRoot = serde_json::Map<String, Value>;

fn read_config() -> io::Result<Option<String>> {
    #[cfg(test)]
    {
        Ok(TEST_FILE.with(|file| file.borrow().clone()))
    }
    #[cfg(not(test))]
    {
        let path = config_path().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "no home directory for config")
        })?;
        read_config_file(&path)
    }
}

fn read_config_file(path: &Path) -> io::Result<Option<String>> {
    if !regular_file_or_absent(path)? {
        return Ok(None);
    }
    match fs::read_to_string(path) {
        Ok(body) => Ok(Some(body)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn parse_root(body: Option<String>) -> io::Result<ConfigRoot> {
    let Some(body) = body else {
        return Ok(ConfigRoot::new());
    };
    let value: Value = serde_json::from_str(&body)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    value
        .as_object()
        .cloned()
        .ok_or_else(|| invalid_config("config root must be a JSON object"))
}

pub(super) fn invalid_config(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

pub(in crate::config) fn load_root() -> Option<Value> {
    let body = read_config().ok().flatten()?;
    let value: Value = serde_json::from_str(&body).ok()?;
    value.is_object().then_some(value)
}

/// A non-blank string at `root[field]`, or `root[section][field]`.
pub(super) fn stored_str(section: Option<&str>, field: &str) -> Option<String> {
    let root = load_root()?;
    let value = match section {
        Some(section) => &root[section][field],
        None => &root[field],
    };
    value
        .as_str()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(String::from)
}

/// Set one string field, creating the file if needed. Other fields in
/// an existing file are preserved.
pub(super) fn save_str(section: Option<&str>, field: &str, value: &str) -> io::Result<PathBuf> {
    let value = json!(value);
    match section {
        Some(section) => mutate_section_checked(section, field, move |entry| {
            if !entry.is_null() && !entry.is_string() {
                return Err(invalid_config(
                    "saved string setting has an incompatible type",
                ));
            }
            *entry = value;
            Ok(())
        }),
        None => {
            let field = field.to_string();
            mutate_root_checked(move |root| {
                if root.get(&field).is_some_and(|saved| !saved.is_string()) {
                    return Err(invalid_config(
                        "saved string setting has an incompatible type",
                    ));
                }
                root.insert(field, value);
                Ok(())
            })
        }
    }
}

/// Load, edit, and rewrite the config, preserving unrelated fields.
pub(in crate::config) fn mutate_root(edit: impl FnOnce(&mut ConfigRoot)) -> io::Result<PathBuf> {
    mutate_root_checked(move |root| {
        edit(root);
        Ok(())
    })
}

pub(super) fn mutate_root_checked(
    edit: impl FnOnce(&mut ConfigRoot) -> io::Result<()>,
) -> io::Result<PathBuf> {
    let path = config_path()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no home directory for config"))?;
    #[cfg(test)]
    {
        let mut root = parse_root(read_config()?)?;
        edit(&mut root)?;
        let body = serialize_root(&root)?;
        TEST_FILE.with(|file| *file.borrow_mut() = Some(body));
        Ok(path)
    }
    #[cfg(not(test))]
    {
        mutate_config_file(&path, edit)?;
        Ok(path)
    }
}

fn serialize_root(root: &ConfigRoot) -> io::Result<String> {
    let mut body = serde_json::to_string_pretty(root)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    body.push('\n');
    Ok(body)
}

/// Edit one value inside a named object section, creating the section only
/// when absent. A malformed existing section is never replaced silently.
pub(in crate::config) fn mutate_section(
    section: &str,
    field: &str,
    edit: impl FnOnce(&mut Value),
) -> io::Result<PathBuf> {
    mutate_section_checked(section, field, move |entry| {
        edit(entry);
        Ok(())
    })
}

pub(super) fn mutate_section_checked(
    section: &str,
    field: &str,
    edit: impl FnOnce(&mut Value) -> io::Result<()>,
) -> io::Result<PathBuf> {
    let field = field.to_string();
    let section = section.to_string();
    mutate_root_checked(move |root| {
        let saved = root.entry(section).or_insert_with(|| json!({}));
        let values = saved
            .as_object_mut()
            .ok_or_else(|| invalid_config("config section must be an object"))?;
        edit(values.entry(field).or_insert(Value::Null))
    })
}

// The persistent sidecar lock survives atomic replacement of config.json,
// so cooperating processes serialize the entire read-modify-write cycle.
pub(super) fn mutate_config_file(
    path: &Path,
    edit: impl FnOnce(&mut ConfigRoot) -> io::Result<()>,
) -> io::Result<()> {
    let _process_guard = CONFIG_WRITE_MUTEX
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let path = prepare_config_path(path)?;
    let _file_guard = lock_config_file(&path)?;
    if regular_file_or_absent(&path)? {
        restrict(&path, 0o600)?;
    }
    let mut root = parse_root(read_config_file(&path)?)?;
    edit(&mut root)?;
    let body = serialize_root(&root)?;
    write_private(&path, &body)
}

pub(super) fn prepare_config_path(path: &Path) -> io::Result<PathBuf> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let parent = fs::canonicalize(parent)?;
    if !fs::metadata(&parent)?.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "config parent must be a directory",
        ));
    }
    restrict(&parent, 0o700)?;
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid config path"))?;
    Ok(parent.join(name))
}

fn regular_file_or_absent(path: &Path) -> io::Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => Ok(true),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "config path must be a regular file, not a link or directory",
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

// Lock a stable sibling file rather than config.json itself: replacing the
// config changes its inode and would otherwise let later writers race.
fn lock_config_file(path: &Path) -> io::Result<File> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid config path"))?;
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid config path"))?
        .to_string_lossy();
    let lock_path = parent.join(format!(".{name}.lock"));
    regular_file_or_absent(&lock_path)?;

    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(&lock_path)?;
    restrict(&lock_path, 0o600)?;
    file.lock()?;
    Ok(file)
}

pub(super) fn write_private(path: &Path, body: &str) -> io::Result<()> {
    write_private_with_hook(path, body, |_| Ok(()))
}

// The temporary file is created and restricted before its first secret byte,
// then synced and atomically moved into place. The hook injects pre-commit
// failures for tests without exposing an unsafe production write path.
pub(super) fn write_private_with_hook(
    path: &Path,
    body: &str,
    before_replace: impl FnOnce(&Path) -> io::Result<()>,
) -> io::Result<()> {
    let path = prepare_config_path(path)?;
    regular_file_or_absent(&path)?;
    #[cfg(unix)]
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid config path"))?;
    let (temp_path, mut file) = create_private_temp(&path)?;
    let result = (|| {
        file.write_all(body.as_bytes())?;
        file.sync_all()?;
        before_replace(&temp_path)?;
        regular_file_or_absent(&path)?;
        atomic_replace(&temp_path, &path)?;
        #[cfg(unix)]
        File::open(parent)?.sync_all()?;
        Ok(())
    })();
    drop(file);
    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

#[cfg(not(windows))]
fn atomic_replace(source: &Path, destination: &Path) -> io::Result<()> {
    fs::rename(source, destination)
}

#[cfg(windows)]
fn atomic_replace(source: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let mut source: Vec<u16> = source.as_os_str().encode_wide().collect();
    source.push(0);
    let mut destination: Vec<u16> = destination.as_os_str().encode_wide().collect();
    destination.push(0);
    let moved = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if moved == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

pub(super) fn create_private_temp(path: &Path) -> io::Result<(PathBuf, File)> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid config path"))?;
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid config path"))?
        .to_string_lossy();
    for _ in 0..32 {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            ^ u128::from(TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed));
        let temp_path = parent.join(format!(".{name}.{}.{}.tmp", std::process::id(), nonce));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&temp_path) {
            Ok(file) => {
                if let Err(error) = restrict(&temp_path, 0o600) {
                    drop(file);
                    let _ = fs::remove_file(&temp_path);
                    return Err(error);
                }
                return Ok((temp_path, file));
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate a unique config temporary file",
    ))
}

#[cfg(unix)]
fn restrict(path: &Path, mode: u32) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
}

#[cfg(windows)]
fn restrict(path: &Path, _mode: u32) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Authorization::{
        ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
    };
    use windows_sys::Win32::Security::{
        SetFileSecurityW, DACL_SECURITY_INFORMATION, PROTECTED_DACL_SECURITY_INFORMATION,
    };

    let sddl: Vec<u16> = "D:P(A;;FA;;;OW)(A;;FA;;;SY)\0".encode_utf16().collect();
    let mut descriptor = std::ptr::null_mut();
    let mut descriptor_size = 0;
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
    let secured = unsafe {
        SetFileSecurityW(
            wide_path.as_ptr(),
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            descriptor,
        )
    };
    let error = (secured == 0).then(io::Error::last_os_error);
    unsafe { LocalFree(descriptor) };
    error.map_or(Ok(()), Err)
}

#[cfg(not(any(unix, windows)))]
fn restrict(_path: &Path, _mode: u32) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "secure config storage is unsupported on this platform",
    ))
}
