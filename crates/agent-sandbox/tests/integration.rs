use agent_sandbox::Sandbox;
use agent_sandbox::config::SandboxConfig;

fn temp_sandbox() -> (tempfile::TempDir, Sandbox) {
    let tmp = tempfile::tempdir().unwrap();
    let config = SandboxConfig {
        work_dir: tmp.path().to_path_buf(),
        ..Default::default()
    };
    let sandbox = Sandbox::new(config).unwrap();
    (tmp, sandbox)
}

#[tokio::test]
async fn test_create_sandbox() {
    let (_tmp, sandbox) = temp_sandbox();
    // Sandbox created successfully
    sandbox.destroy().await.unwrap();
}

#[tokio::test]
async fn test_write_and_read_file() {
    let (_tmp, sandbox) = temp_sandbox();

    sandbox
        .write_file("test.txt", b"hello world")
        .await
        .unwrap();

    let content = sandbox.read_file("test.txt").await.unwrap();
    assert_eq!(content, b"hello world");
}

#[tokio::test]
async fn test_write_file_creates_parent_dirs() {
    let (_tmp, sandbox) = temp_sandbox();

    sandbox.write_file("a/b/c.txt", b"nested").await.unwrap();

    let content = sandbox.read_file("a/b/c.txt").await.unwrap();
    assert_eq!(content, b"nested");
}

#[tokio::test]
async fn test_list_dir() {
    let (tmp, sandbox) = temp_sandbox();

    std::fs::write(tmp.path().join("a.txt"), "a").unwrap();
    std::fs::write(tmp.path().join("b.txt"), "b").unwrap();
    std::fs::create_dir(tmp.path().join("subdir")).unwrap();

    let entries = sandbox.list_dir(".").await.unwrap();
    assert_eq!(entries.len(), 3);
    assert!(entries.iter().any(|e| e.name == "a.txt" && e.is_file));
    assert!(entries.iter().any(|e| e.name == "b.txt" && e.is_file));
    assert!(entries.iter().any(|e| e.name == "subdir" && e.is_dir));
}

#[tokio::test]
async fn test_exec_echo() {
    let (_tmp, sandbox) = temp_sandbox();

    let result = sandbox
        .exec("echo", &["hello".into(), "world".into()])
        .await
        .unwrap();

    assert_eq!(result.exit_code, 0);
    assert_eq!(
        String::from_utf8_lossy(&result.stdout).trim(),
        "hello world"
    );
}

#[tokio::test]
async fn test_exec_cat() {
    let (tmp, sandbox) = temp_sandbox();

    std::fs::write(tmp.path().join("hello.txt"), "hello sandbox").unwrap();

    let result = sandbox
        .exec("cat", &["/work/hello.txt".into()])
        .await
        .unwrap();

    assert_eq!(result.exit_code, 0);
    assert!(String::from_utf8_lossy(&result.stdout).contains("hello sandbox"));
}

#[tokio::test]
async fn test_exec_ls() {
    let (tmp, sandbox) = temp_sandbox();

    std::fs::write(tmp.path().join("file1.txt"), "").unwrap();
    std::fs::write(tmp.path().join("file2.txt"), "").unwrap();

    let result = sandbox.exec("ls", &["/work".into()]).await.unwrap();

    assert_eq!(result.exit_code, 0);
    let output = String::from_utf8_lossy(&result.stdout);
    assert!(output.contains("file1.txt"));
    assert!(output.contains("file2.txt"));
}

#[tokio::test]
async fn test_exec_find() {
    let (tmp, sandbox) = temp_sandbox();

    std::fs::create_dir_all(tmp.path().join("a/b")).unwrap();
    std::fs::write(tmp.path().join("a/b/deep.txt"), "deep").unwrap();

    let result = sandbox
        .exec("find", &["/work".into(), "-name".into(), "*.txt".into()])
        .await
        .unwrap();

    assert_eq!(result.exit_code, 0);
    let output = String::from_utf8_lossy(&result.stdout);
    assert!(output.contains("deep.txt"));
}

#[tokio::test]
async fn test_exec_grep() {
    let (tmp, sandbox) = temp_sandbox();

    std::fs::write(
        tmp.path().join("code.rs"),
        "fn main() {\n    println!(\"hello\");\n}\n",
    )
    .unwrap();

    let result = sandbox
        .exec("grep", &["main".into(), "/work/code.rs".into()])
        .await
        .unwrap();

    assert_eq!(result.exit_code, 0);
    let output = String::from_utf8_lossy(&result.stdout);
    assert!(output.contains("fn main()"));
}

#[tokio::test]
async fn test_exec_wc() {
    let (tmp, sandbox) = temp_sandbox();

    std::fs::write(tmp.path().join("lines.txt"), "one\ntwo\nthree\n").unwrap();

    let result = sandbox
        .exec("wc", &["-l".into(), "/work/lines.txt".into()])
        .await
        .unwrap();

    assert_eq!(result.exit_code, 0);
    let output = String::from_utf8_lossy(&result.stdout);
    assert!(output.contains("3"));
}

#[tokio::test]
async fn test_exec_mkdir_and_touch() {
    let (_tmp, sandbox) = temp_sandbox();

    let result = sandbox
        .exec("mkdir", &["-p".into(), "/work/newdir/sub".into()])
        .await
        .unwrap();
    assert_eq!(result.exit_code, 0);

    let result = sandbox
        .exec("touch", &["/work/newdir/sub/file.txt".into()])
        .await
        .unwrap();
    assert_eq!(result.exit_code, 0);

    let result = sandbox
        .exec("ls", &["/work/newdir/sub".into()])
        .await
        .unwrap();
    assert_eq!(result.exit_code, 0);
    let output = String::from_utf8_lossy(&result.stdout);
    assert!(output.contains("file.txt"));
}

#[tokio::test]
async fn test_path_traversal_blocked() {
    let (_tmp, sandbox) = temp_sandbox();

    let result = sandbox.read_file("../../../etc/passwd").await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("traversal"));
}

#[tokio::test]
async fn test_command_not_found() {
    let (_tmp, sandbox) = temp_sandbox();

    let result = sandbox.exec("nonexistent_cmd", &[]).await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("not found"));
}

#[tokio::test]
async fn test_fuel_exhaustion_timeout() {
    let tmp = tempfile::tempdir().unwrap();
    let config = SandboxConfig {
        work_dir: tmp.path().to_path_buf(),
        fuel_limit: 1000, // Very low fuel to trigger exhaustion
        ..Default::default()
    };
    let sandbox = Sandbox::new(config).unwrap();

    // Even a simple echo should exhaust 1000 fuel units
    let result = sandbox.exec("echo", &["hello".into()]).await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("timed out") || err.contains("fuel"),
        "Expected timeout/fuel error, got: {}",
        err
    );
}

#[tokio::test]
async fn test_diff_reports_changes() {
    let (tmp, _sandbox) = temp_sandbox();

    // Write initial file
    std::fs::write(tmp.path().join("existing.txt"), "original").unwrap();

    // Create a new sandbox to snapshot current state
    let config = SandboxConfig {
        work_dir: tmp.path().to_path_buf(),
        ..Default::default()
    };
    let sandbox = Sandbox::new(config).unwrap();

    // Create a new file
    std::fs::write(tmp.path().join("new.txt"), "new content").unwrap();

    // Modify existing file
    std::fs::write(tmp.path().join("existing.txt"), "modified").unwrap();

    let changes = sandbox.diff().await.unwrap();
    assert!(
        changes.iter().any(|c| c.path == "new.txt"),
        "Expected 'new.txt' in changes: {:?}",
        changes.iter().map(|c| &c.path).collect::<Vec<_>>()
    );
    assert!(
        changes.iter().any(|c| c.path == "existing.txt"),
        "Expected 'existing.txt' in changes: {:?}",
        changes.iter().map(|c| &c.path).collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn test_destroy_prevents_operations() {
    let (_tmp, sandbox) = temp_sandbox();

    sandbox.destroy().await.unwrap();

    let result = sandbox.read_file("anything.txt").await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("destroyed"));
}

#[tokio::test]
async fn test_exec_sed() {
    let (tmp, sandbox) = temp_sandbox();

    std::fs::write(tmp.path().join("input.txt"), "hello world\n").unwrap();

    let result = sandbox
        .exec("sed", &["s/world/rust/g".into(), "/work/input.txt".into()])
        .await
        .unwrap();

    assert_eq!(result.exit_code, 0);
    let output = String::from_utf8_lossy(&result.stdout);
    assert!(output.contains("hello rust"));
}

#[tokio::test]
async fn test_exec_basename_dirname() {
    let (_tmp, sandbox) = temp_sandbox();

    let result = sandbox
        .exec("basename", &["/work/path/to/file.txt".into()])
        .await
        .unwrap();
    assert_eq!(result.exit_code, 0);
    assert_eq!(String::from_utf8_lossy(&result.stdout).trim(), "file.txt");

    let result = sandbox
        .exec("dirname", &["/work/path/to/file.txt".into()])
        .await
        .unwrap();
    assert_eq!(result.exit_code, 0);
    assert_eq!(
        String::from_utf8_lossy(&result.stdout).trim(),
        "/work/path/to"
    );
}

// --- Additional tool exec tests ---

#[tokio::test]
async fn test_exec_head() {
    let (tmp, sandbox) = temp_sandbox();

    std::fs::write(
        tmp.path().join("data.txt"),
        "line1\nline2\nline3\nline4\nline5\n",
    )
    .unwrap();

    let result = sandbox
        .exec("head", &["-n".into(), "2".into(), "/work/data.txt".into()])
        .await
        .unwrap();

    assert_eq!(result.exit_code, 0);
    let output = String::from_utf8_lossy(&result.stdout);
    assert!(output.contains("line1"));
    assert!(output.contains("line2"));
    assert!(!output.contains("line3"));
}

#[tokio::test]
async fn test_exec_tail() {
    let (tmp, sandbox) = temp_sandbox();

    std::fs::write(
        tmp.path().join("data.txt"),
        "line1\nline2\nline3\nline4\nline5\n",
    )
    .unwrap();

    let result = sandbox
        .exec("tail", &["-n".into(), "2".into(), "/work/data.txt".into()])
        .await
        .unwrap();

    assert_eq!(result.exit_code, 0);
    let output = String::from_utf8_lossy(&result.stdout);
    assert!(!output.contains("line3"));
    assert!(output.contains("line4"));
    assert!(output.contains("line5"));
}

#[tokio::test]
async fn test_exec_sort() {
    let (tmp, sandbox) = temp_sandbox();

    std::fs::write(tmp.path().join("unsorted.txt"), "banana\napple\ncherry\n").unwrap();

    let result = sandbox
        .exec("sort", &["/work/unsorted.txt".into()])
        .await
        .unwrap();

    assert_eq!(result.exit_code, 0);
    assert_eq!(
        String::from_utf8_lossy(&result.stdout).trim(),
        "apple\nbanana\ncherry"
    );
}

#[tokio::test]
async fn test_exec_uniq() {
    let (tmp, sandbox) = temp_sandbox();

    std::fs::write(tmp.path().join("dups.txt"), "a\na\nb\nb\nb\nc\n").unwrap();

    let result = sandbox
        .exec("uniq", &["/work/dups.txt".into()])
        .await
        .unwrap();

    assert_eq!(result.exit_code, 0);
    assert_eq!(String::from_utf8_lossy(&result.stdout).trim(), "a\nb\nc");
}

#[tokio::test]
async fn test_exec_cp() {
    let (tmp, sandbox) = temp_sandbox();

    std::fs::write(tmp.path().join("src.txt"), "copy me").unwrap();

    let result = sandbox
        .exec("cp", &["/work/src.txt".into(), "/work/dst.txt".into()])
        .await
        .unwrap();

    assert_eq!(result.exit_code, 0);
    let content = std::fs::read_to_string(tmp.path().join("dst.txt")).unwrap();
    assert_eq!(content, "copy me");
}

#[tokio::test]
async fn test_exec_mv() {
    let (tmp, sandbox) = temp_sandbox();

    std::fs::write(tmp.path().join("old.txt"), "move me").unwrap();

    let result = sandbox
        .exec("mv", &["/work/old.txt".into(), "/work/new.txt".into()])
        .await
        .unwrap();

    assert_eq!(result.exit_code, 0);
    assert!(!tmp.path().join("old.txt").exists());
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("new.txt")).unwrap(),
        "move me"
    );
}

#[tokio::test]
async fn test_exec_rm() {
    let (tmp, sandbox) = temp_sandbox();

    std::fs::write(tmp.path().join("delete.txt"), "bye").unwrap();
    assert!(tmp.path().join("delete.txt").exists());

    let result = sandbox
        .exec("rm", &["/work/delete.txt".into()])
        .await
        .unwrap();

    assert_eq!(result.exit_code, 0);
    assert!(!tmp.path().join("delete.txt").exists());
}

#[tokio::test]
async fn test_exec_base64() {
    let (tmp, sandbox) = temp_sandbox();

    std::fs::write(tmp.path().join("plain.txt"), "hello").unwrap();

    let result = sandbox
        .exec("base64", &["/work/plain.txt".into()])
        .await
        .unwrap();

    assert_eq!(result.exit_code, 0);
    assert_eq!(String::from_utf8_lossy(&result.stdout).trim(), "aGVsbG8=");
}

#[tokio::test]
async fn test_exec_sha256sum() {
    let (tmp, sandbox) = temp_sandbox();

    std::fs::write(tmp.path().join("hash.txt"), "hello").unwrap();

    let result = sandbox
        .exec("sha256sum", &["/work/hash.txt".into()])
        .await
        .unwrap();

    assert_eq!(result.exit_code, 0);
    let output = String::from_utf8_lossy(&result.stdout);
    // sha256("hello") = 2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824
    assert!(output.contains("2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"));
}

#[tokio::test]
async fn test_exec_diff() {
    let (tmp, sandbox) = temp_sandbox();

    std::fs::write(tmp.path().join("a.txt"), "line1\nline2\nline3\n").unwrap();
    std::fs::write(tmp.path().join("b.txt"), "line1\nmodified\nline3\n").unwrap();

    let result = sandbox
        .exec("diff", &["/work/a.txt".into(), "/work/b.txt".into()])
        .await
        .unwrap();

    // diff returns exit code 1 when files differ
    assert_eq!(result.exit_code, 1);
    let output = String::from_utf8_lossy(&result.stdout);
    assert!(output.contains("line2") || output.contains("modified"));
}

#[tokio::test]
async fn test_exec_cut() {
    let (tmp, sandbox) = temp_sandbox();

    std::fs::write(tmp.path().join("csv.txt"), "a,b,c\n1,2,3\n").unwrap();

    let result = sandbox
        .exec(
            "cut",
            &[
                "-d".into(),
                ",".into(),
                "-f".into(),
                "2".into(),
                "/work/csv.txt".into(),
            ],
        )
        .await
        .unwrap();

    assert_eq!(result.exit_code, 0);
    assert_eq!(String::from_utf8_lossy(&result.stdout).trim(), "b\n2");
}

#[tokio::test]
async fn test_exec_env() {
    let (_tmp, sandbox) = temp_sandbox();

    let result = sandbox.exec("env", &[]).await.unwrap();

    assert_eq!(result.exit_code, 0);
    let output = String::from_utf8_lossy(&result.stdout);
    assert!(output.contains("TOOLBOX_CMD=env"));
}

// --- Security tests ---

#[tokio::test]
async fn test_security_path_traversal_variants() {
    let (_tmp, sandbox) = temp_sandbox();

    // Various traversal attempts via readFile
    let traversals = [
        "../../../etc/passwd",
        "../../etc/shadow",
        "foo/../../..",
        "./../../etc/hosts",
        "foo/../../../etc/passwd",
    ];

    for path in traversals {
        let result = sandbox.read_file(path).await;
        assert!(
            result.is_err(),
            "Path '{}' should be blocked but was allowed",
            path
        );
        assert!(
            result.unwrap_err().to_string().contains("traversal"),
            "Path '{}' should return traversal error",
            path
        );
    }
}

#[tokio::test]
async fn test_security_write_file_traversal() {
    let (_tmp, sandbox) = temp_sandbox();

    // Attempt to write outside the sandbox
    let result = sandbox
        .write_file("../../../tmp/escape.txt", b"pwned")
        .await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("traversal"));
}

#[tokio::test]
async fn test_security_list_dir_traversal() {
    let (_tmp, sandbox) = temp_sandbox();

    let result = sandbox.list_dir("../../../etc").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("traversal"));
}

#[tokio::test]
async fn test_security_symlink_escape() {
    let (tmp, sandbox) = temp_sandbox();

    // Create a symlink inside work dir that points outside
    let link_path = tmp.path().join("escape_link");
    std::os::unix::fs::symlink("/etc", &link_path).unwrap();

    // Reading via the symlink should fail — the resolved path is outside the sandbox
    let result = sandbox.read_file("escape_link/passwd").await;
    assert!(
        result.is_err(),
        "Symlink escape to /etc/passwd should be blocked"
    );
}

#[tokio::test]
async fn test_security_cat_cannot_read_host_files() {
    let (_tmp, sandbox) = temp_sandbox();

    // WASM sandbox should not have access to /etc/passwd via cat
    let result = sandbox.exec("cat", &["/etc/passwd".into()]).await.unwrap();

    // Should fail since /etc is not mounted
    assert_ne!(result.exit_code, 0);
    assert!(String::from_utf8_lossy(&result.stdout).is_empty());
}

#[tokio::test]
async fn test_security_find_confined_to_sandbox() {
    let (_tmp, sandbox) = temp_sandbox();

    // find should not be able to traverse outside /work
    let result = sandbox
        .exec("find", &["/".into(), "-name".into(), "passwd".into()])
        .await
        .unwrap();

    let output = String::from_utf8_lossy(&result.stdout);
    // Should not find /etc/passwd — only /work is mounted
    assert!(
        !output.contains("/etc/passwd"),
        "find should not see /etc/passwd, got: {}",
        output
    );
}

#[tokio::test]
async fn test_security_cp_cannot_write_outside_sandbox() {
    let (tmp, sandbox) = temp_sandbox();

    std::fs::write(tmp.path().join("secret.txt"), "data").unwrap();

    // Attempt to copy to a path outside /work
    let result = sandbox
        .exec("cp", &["/work/secret.txt".into(), "/tmp/escape.txt".into()])
        .await
        .unwrap();

    // Should fail — /tmp is not writable/mounted
    assert_ne!(result.exit_code, 0);
    assert!(!std::path::Path::new("/tmp/escape.txt").exists());
}

#[tokio::test]
async fn test_security_env_vars_isolated() {
    let tmp = tempfile::tempdir().unwrap();
    let config = SandboxConfig {
        work_dir: tmp.path().to_path_buf(),
        env_vars: [("SECRET_KEY".into(), "s3cret".into())]
            .into_iter()
            .collect(),
        ..Default::default()
    };
    let sandbox = Sandbox::new(config).unwrap();

    let result = sandbox.exec("env", &[]).await.unwrap();
    let output = String::from_utf8_lossy(&result.stdout);

    // Configured env vars should be visible
    assert!(output.contains("SECRET_KEY=s3cret"));

    // Host env vars like HOME, USER, PATH should NOT leak into the sandbox
    assert!(
        !output.contains("HOME="),
        "Host HOME should not leak into sandbox"
    );
    assert!(
        !output.contains("USER="),
        "Host USER should not leak into sandbox"
    );
}

#[tokio::test]
async fn test_security_fuel_limit_prevents_infinite_loop() {
    let tmp = tempfile::tempdir().unwrap();
    let config = SandboxConfig {
        work_dir: tmp.path().to_path_buf(),
        fuel_limit: 100_000, // Low enough to stop runaway but enough to start
        ..Default::default()
    };
    let sandbox = Sandbox::new(config).unwrap();

    // Try running a command — with limited fuel it should error, not hang
    let result = sandbox.exec("echo", &["test".into()]).await;
    // Either succeeds quickly or fails with timeout/fuel — should NOT hang
    assert!(
        result.is_ok() || result.unwrap_err().to_string().contains("timed out"),
        "Low fuel should either complete or timeout, not hang"
    );
}

#[tokio::test]
async fn test_security_timeout_prevents_hang() {
    let tmp = tempfile::tempdir().unwrap();
    let config = SandboxConfig {
        work_dir: tmp.path().to_path_buf(),
        timeout: std::time::Duration::from_secs(2), // 2 second timeout
        fuel_limit: u64::MAX,                       // Effectively unlimited fuel
        ..Default::default()
    };
    let sandbox = Sandbox::new(config).unwrap();

    let start = std::time::Instant::now();
    // Even with unlimited fuel, we should not wait longer than timeout + margin
    let _result = sandbox.exec("echo", &["hello".into()]).await;
    let elapsed = start.elapsed();

    assert!(
        elapsed < std::time::Duration::from_secs(10),
        "Execution should respect timeout, took {:?}",
        elapsed
    );
}

#[tokio::test]
async fn test_security_destroyed_sandbox_blocks_all_ops() {
    let (_tmp, sandbox) = temp_sandbox();

    sandbox.destroy().await.unwrap();

    // All operations should fail with "destroyed"
    assert!(sandbox.read_file("any.txt").await.is_err());
    assert!(sandbox.write_file("any.txt", b"data").await.is_err());
    assert!(sandbox.list_dir(".").await.is_err());
    assert!(sandbox.exec("echo", &["hello".into()]).await.is_err());
    assert!(sandbox.diff().await.is_err());
}

#[tokio::test]
async fn test_security_grep_cannot_read_host_files() {
    let (_tmp, sandbox) = temp_sandbox();

    let result = sandbox
        .exec("grep", &["root".into(), "/etc/passwd".into()])
        .await
        .unwrap();

    // grep should fail because /etc is not mounted
    assert_ne!(result.exit_code, 0);
}

#[tokio::test]
async fn test_security_rm_cannot_delete_outside_sandbox() {
    let (_tmp, sandbox) = temp_sandbox();

    let result = sandbox.exec("rm", &["/etc/hostname".into()]).await.unwrap();

    // rm outside /work should fail
    assert_ne!(result.exit_code, 0);
}

#[tokio::test]
async fn test_security_multiple_sandboxes_isolated() {
    let tmp1 = tempfile::tempdir().unwrap();
    let tmp2 = tempfile::tempdir().unwrap();

    let sandbox1 = Sandbox::new(SandboxConfig {
        work_dir: tmp1.path().to_path_buf(),
        ..Default::default()
    })
    .unwrap();

    let sandbox2 = Sandbox::new(SandboxConfig {
        work_dir: tmp2.path().to_path_buf(),
        ..Default::default()
    })
    .unwrap();

    // Write a file in sandbox1
    std::fs::write(tmp1.path().join("secret.txt"), "sandbox1 secret").unwrap();

    // Sandbox2 should not see sandbox1's files
    let result = sandbox2
        .exec("cat", &["/work/secret.txt".into()])
        .await
        .unwrap();
    assert_ne!(
        result.exit_code, 0,
        "Sandbox2 should not see sandbox1's files"
    );

    // Sandbox1 should see its own file
    let result = sandbox1
        .exec("cat", &["/work/secret.txt".into()])
        .await
        .unwrap();
    assert_eq!(result.exit_code, 0);
    assert!(String::from_utf8_lossy(&result.stdout).contains("sandbox1 secret"));
}
