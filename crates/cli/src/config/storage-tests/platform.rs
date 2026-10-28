    #[cfg(unix)]
    #[test]
    fn config_symlink_target_is_rejected_without_touching_referent() {
        let dir = temp_config_dir("symlink");
        fs::create_dir_all(&dir).unwrap();
        let target = dir.join("real.json");
        let link = dir.join("config.json");
        fs::write(&target, b"preserve").unwrap();
        std::os::unix::fs::symlink(&target, &link).unwrap();

        assert!(write_private(&link, "new config").is_err());
        assert_eq!(fs::read(&target).unwrap(), b"preserve");
        assert!(fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink());
        fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn config_symlink_target_is_rejected_without_touching_referent() {
        let dir = temp_config_dir("symlink");
        fs::create_dir_all(&dir).unwrap();
        let target = dir.join("real.json");
        let link = dir.join("config.json");
        fs::write(&target, b"preserve").unwrap();
        std::os::windows::fs::symlink_file(&target, &link)
            .expect("create symlink fixture for config protection test");
        assert!(write_private(&link, "new config").is_err());
        assert_eq!(fs::read(&target).unwrap(), b"preserve");
        assert!(fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn concurrent_process_mutations_preserve_both_updates() {
        let dir = temp_config_dir("concurrent");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        fs::write(&path, "{}\n").unwrap();
        let start = dir.join("start");
        let ready_a = dir.join("ready-a");
        let ready_b = dir.join("ready-b");
        let executable = std::env::current_exe().unwrap();

        let spawn = |key: &str, ready: &Path| {
            Command::new(&executable)
                .args([
                    "--exact",
                    "config::storage::tests::config_writer_child_process",
                    "--nocapture",
                ])
                .env("FINECORE_CONFIG_CHILD_PATH", &path)
                .env("FINECORE_CONFIG_CHILD_KEY", key)
                .env("FINECORE_CONFIG_CHILD_READY", ready)
                .env("FINECORE_CONFIG_CHILD_START", &start)
                .spawn()
                .unwrap()
        };
        let mut child_a = spawn("writer-a", &ready_a);
        let mut child_b = spawn("writer-b", &ready_b);
        let deadline = Instant::now() + Duration::from_secs(10);
        while !(ready_a.exists() && ready_b.exists()) {
            assert!(
                Instant::now() < deadline,
                "writer children did not become ready"
            );
            thread::sleep(Duration::from_millis(5));
        }
        fs::write(&start, b"go").unwrap();
        assert!(child_a.wait().unwrap().success());
        assert!(child_b.wait().unwrap().success());

        let saved: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(saved["api_keys"]["writer-a"], "writer-a");
        assert_eq!(saved["api_keys"]["writer-b"], "writer-b");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn config_writer_child_process() {
        let (Ok(path), Ok(key), Ok(ready), Ok(start)) = (
            std::env::var("FINECORE_CONFIG_CHILD_PATH"),
            std::env::var("FINECORE_CONFIG_CHILD_KEY"),
            std::env::var("FINECORE_CONFIG_CHILD_READY"),
            std::env::var("FINECORE_CONFIG_CHILD_START"),
        ) else {
            return;
        };
        fs::write(&ready, b"ready").unwrap();
        let start = PathBuf::from(start);
        let deadline = Instant::now() + Duration::from_secs(10);
        while !start.exists() {
            assert!(Instant::now() < deadline, "writer start barrier timed out");
            thread::sleep(Duration::from_millis(5));
        }
        mutate_config_file(Path::new(&path), move |root| {
            let section = root
                .entry("api_keys")
                .or_insert_with(|| json!({}))
                .as_object_mut()
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidData, "api_keys is not an object")
                })?;
            section.insert(key.clone(), json!(key));
            thread::sleep(Duration::from_millis(200));
            Ok(())
        })
        .unwrap();
    }

