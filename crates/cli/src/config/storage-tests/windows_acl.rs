    #[cfg(unix)]
    #[test]
    fn existing_config_is_restricted_before_read_modify_write() {
        use std::os::unix::fs::PermissionsExt;

        let dir = temp_config_dir("tighten-existing");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        fs::write(&path, r#"{"api_keys":{}}"#).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();

        mutate_config_file(&path, |_| {
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
            Ok(())
        })
        .unwrap();
        fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn config_file_and_temp_are_restricted_before_body_is_written() {
        use std::os::unix::fs::PermissionsExt;

        let dir = temp_config_dir("temp-permissions");
        let path = prepare_config_path(&dir.join("config.json")).unwrap();
        let (temp, file) = create_private_temp(&path).unwrap();
        assert_eq!(file.metadata().unwrap().len(), 0);
        assert_eq!(
            fs::metadata(&dir).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(&temp).unwrap().permissions().mode() & 0o777,
            0o600
        );
        drop(file);
        fs::remove_file(temp).unwrap();
        fs::remove_dir_all(dir).unwrap();
    }

    /// The real on-disk writer must produce owner-only files and tighten a
    /// pre-existing looser file by replacing it with a protected inode.
    #[cfg(unix)]
    #[test]
    fn config_file_is_written_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("orcacode-perm-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");

        write_private(&path, "{}\n").unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "config must not be group/world readable");

        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        write_private(&path, "{}\n").unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "rewrites tighten a loose pre-existing file");

        assert_eq!(
            fs::metadata(&dir).unwrap().permissions().mode() & 0o777,
            0o700
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn existing_config_acl_is_restricted_before_read_modify_write() {
        let dir = temp_config_dir("tighten-existing");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        fs::write(&path, r#"{"api_keys":{}}"#).unwrap();
        let grant = std::process::Command::new("icacls")
            .arg(&path)
            .args(["/grant", "*S-1-1-0:F"])
            .output()
            .unwrap();
        assert!(
            grant.status.success(),
            "{}",
            String::from_utf8_lossy(&grant.stderr)
        );

        mutate_config_file(&path, |_| {
            let acl = std::process::Command::new("icacls")
                .arg(&path)
                .output()
                .unwrap();
            assert!(acl.status.success());
            assert_eq!(
                String::from_utf8_lossy(&acl.stdout).matches(":(F)").count(),
                2
            );
            Ok(())
        })
        .unwrap();
        fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn config_directory_temp_and_file_have_owner_system_only_acl() {
        let dir = temp_config_dir("windows-acl");
        let path = prepare_config_path(&dir.join("config.json")).unwrap();
        let (temp, file) = create_private_temp(&path).unwrap();
        assert_eq!(file.metadata().unwrap().len(), 0);
        let temp_acl = std::process::Command::new("icacls")
            .arg(&temp)
            .output()
            .expect("icacls must be available on Windows");
        assert!(temp_acl.status.success());
        let temp_acl = String::from_utf8_lossy(&temp_acl.stdout);
        assert_eq!(temp_acl.matches(":(F)").count(), 2, "{temp_acl}");
        drop(file);
        fs::remove_file(&temp).unwrap();

        let path = dir.join("config.json");
        write_private(&path, "fixture-only").unwrap();
        let file_acl = std::process::Command::new("icacls")
            .arg(&path)
            .output()
            .unwrap();
        let dir_acl = std::process::Command::new("icacls")
            .arg(&dir)
            .output()
            .unwrap();
        assert!(file_acl.status.success());
        assert!(dir_acl.status.success());
        assert_eq!(
            String::from_utf8_lossy(&file_acl.stdout)
                .matches(":(F)")
                .count(),
            2
        );
        assert_eq!(
            String::from_utf8_lossy(&dir_acl.stdout)
                .matches(":(F)")
                .count(),
            2
        );
        fs::remove_dir_all(dir).unwrap();
    }
