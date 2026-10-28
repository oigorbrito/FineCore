    #[test]
    fn disk_mutation_preserves_unknown_fields() {
        let dir = temp_config_dir("unknown-fields");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        fs::write(
            &path,
            r#"{"future":{"kept":true},"api_keys":{"openai":"old"}}"#,
        )
        .unwrap();

        mutate_config_file(&path, |root| {
            let keys = root
                .entry("api_keys")
                .or_insert_with(|| json!({}))
                .as_object_mut()
                .ok_or_else(|| invalid_config("api_keys must be an object"))?;
            keys.insert("openrouter".into(), json!("new"));
            Ok(())
        })
        .unwrap();

        let saved: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(saved["future"]["kept"], true);
        assert_eq!(saved["api_keys"]["openai"], "old");
        assert_eq!(saved["api_keys"]["openrouter"], "new");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn disk_mutation_keeps_corrupt_bytes_unchanged() {
        let dir = temp_config_dir("corrupt-disk");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        let original = b"not json {";
        fs::write(&path, original).unwrap();

        assert_eq!(
            mutate_config_file(&path, |_| Ok(())).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(fs::read(&path).unwrap(), original);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn disk_read_errors_are_not_treated_as_missing_config() {
        let dir = temp_config_dir("read-error-bytes");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        let original = [b'{', 0xff, b'}'];
        fs::write(&path, original).unwrap();

        assert_eq!(
            mutate_config_file(&path, |_| Ok(())).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(fs::read(&path).unwrap(), original);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn atomic_replace_replaces_existing_content_and_leaves_no_temp() {
        let dir = temp_config_dir("replace");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        fs::write(&path, "old config").unwrap();

        write_private(&path, "new config").unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "new config");
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn failed_atomic_replace_keeps_the_previous_config_and_cleans_temp() {
        let dir = temp_config_dir("interrupt");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        let original = r#"{"api_keys":{"openai":"old"},"future":true}"#;
        fs::write(&path, original).unwrap();

        let error = write_private_with_hook(&path, r#"{"api_keys":{"openai":"new"}}"#, |_| {
            Err(io::Error::other("injected interruption before replace"))
        })
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::Other);
        assert_eq!(fs::read_to_string(&path).unwrap(), original);
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn config_read_error_is_not_treated_as_a_missing_file() {
        let dir = temp_config_dir("read-error");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        fs::create_dir(&path).unwrap();
        let sentinel = path.join("keep.txt");
        fs::write(&sentinel, b"preserve").unwrap();

        assert!(mutate_config_file(&path, |_| Ok(())).is_err());
        assert_eq!(fs::read(&sentinel).unwrap(), b"preserve");
        assert!(dir.join(".config.json.lock").is_file());
        fs::remove_dir_all(dir).unwrap();
    }

