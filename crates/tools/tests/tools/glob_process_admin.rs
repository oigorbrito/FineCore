// ---- glob -----------------------------------------------------------------

#[tokio::test]
async fn glob_matches_bare_anchored_and_recursive_patterns() {
    let (ws, dir) = temp_ws();
    let write = WriteFileTool::new(ws.clone());
    for (path, content) in [
        ("src/a.rs", "x"),
        ("src/deep/b.rs", "x"),
        ("c.txt", "x"),
        ("src/note.md", "x"),
    ] {
        write
            .call(json!({"path": path, "content": content}), &ctx())
            .await
            .unwrap();
    }
    let glob = GlobTool::new(ws.clone());

    // Bare pattern searches the whole tree.
    let out = glob.call(json!({"pattern": "*.rs"}), &ctx()).await.unwrap();
    assert_eq!(
        out["matches"],
        json!(["src/a.rs", "src/deep/b.rs"]),
        "sorted, tree-wide"
    );

    // Anchored single-star stays at one level.
    let out = glob
        .call(json!({"pattern": "src/*.rs"}), &ctx())
        .await
        .unwrap();
    assert_eq!(out["matches"], json!(["src/a.rs"]));

    // `**` spans levels (including zero).
    let out = glob
        .call(json!({"pattern": "src/**/*.rs"}), &ctx())
        .await
        .unwrap();
    assert_eq!(out["matches"], json!(["src/a.rs", "src/deep/b.rs"]));
    assert_eq!(out["truncated"], json!(false));
    assert!(
        out.get("skippedDirs").is_none(),
        "a clean walk reports no skips"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// An unreadable directory must be counted and reported, never silently
/// dropped: the caller has to be able to tell "no match" from "could
/// not look".
#[cfg(unix)]
#[tokio::test]
async fn glob_counts_unreadable_directories_instead_of_dropping_them() {
    use std::os::unix::fs::PermissionsExt;

    let (ws, dir) = temp_ws();
    let write = WriteFileTool::new(ws.clone());
    for path in ["open/a.rs", "sealed/b.rs"] {
        write
            .call(json!({"path": path, "content": "x"}), &ctx())
            .await
            .unwrap();
    }
    let sealed = dir.join("sealed");
    std::fs::set_permissions(&sealed, std::fs::Permissions::from_mode(0o000)).unwrap();
    if std::fs::read_dir(&sealed).is_ok() {
        // Running as root: permission bits don't bite, the scenario
        // cannot be built. Restore and bail.
        std::fs::set_permissions(&sealed, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::remove_dir_all(&dir).ok();
        return;
    }

    let glob = GlobTool::new(ws.clone());
    let out = glob.call(json!({"pattern": "*.rs"}), &ctx()).await.unwrap();
    assert_eq!(out["matches"], json!(["open/a.rs"]));
    assert_eq!(out["skippedDirs"], json!(1), "the sealed dir is reported");

    std::fs::set_permissions(&sealed, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::remove_dir_all(&dir).ok();
}

// ---- process --------------------------------------------------------------

include!("process_admin.rs");
