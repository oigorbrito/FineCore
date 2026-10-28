#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::plugins::*;
    use std::{
        process::Command,
        thread,
        time::{Duration, Instant, SystemTime, UNIX_EPOCH},
    };

    fn raw() -> String {
        TEST_FILE
            .with(|file| file.borrow().clone())
            .expect("config written")
    }

    fn seed(body: &str) {
        TEST_FILE.with(|file| *file.borrow_mut() = Some(body.to_string()));
    }

    fn temp_config_dir(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "finecore-config-integrity-{label}-{}-{nonce}",
            std::process::id()
        ))
    }


    include!("storage-tests/settings.rs");
    include!("storage-tests/disk.rs");
    include!("storage-tests/platform.rs");
    include!("storage-tests/windows_acl.rs");
}
