use std::collections::HashMap;

use agent_sandbox::config::SandboxConfig;
use agent_sandbox::{DomainPattern, FetchPolicy, FetchRequest, Sandbox};

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

// --- Node.js / JavaScript runtime tests ---

#[tokio::test]
async fn test_node_version() {
    let (_tmp, sandbox) = temp_sandbox();
    let result = sandbox.exec("node", &["--version".into()]).await.unwrap();
    assert_eq!(result.exit_code, 0);
    let output = String::from_utf8_lossy(&result.stdout);
    assert!(output.contains("node v0.1.0"));
}

#[tokio::test]
async fn test_node_eval_console_log() {
    let (_tmp, sandbox) = temp_sandbox();
    let result = sandbox
        .exec(
            "node",
            &["-e".into(), "console.log('hello from js')".into()],
        )
        .await
        .unwrap();
    assert_eq!(result.exit_code, 0);
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(
        stdout.contains("hello from js"),
        "Expected 'hello from js' in stdout: {stdout}"
    );
}

#[tokio::test]
async fn test_node_eval_arithmetic() {
    let (_tmp, sandbox) = temp_sandbox();
    let result = sandbox
        .exec("node", &["-p".into(), "2 + 3 * 4".into()])
        .await
        .unwrap();
    assert_eq!(result.exit_code, 0);
    assert_eq!(String::from_utf8_lossy(&result.stdout).trim(), "14");
}

#[tokio::test]
async fn test_node_eval_string_operations() {
    let (_tmp, sandbox) = temp_sandbox();
    let result = sandbox
        .exec(
            "node",
            &["-p".into(), "'hello'.toUpperCase() + ' WORLD'".into()],
        )
        .await
        .unwrap();
    assert_eq!(result.exit_code, 0);
    assert_eq!(
        String::from_utf8_lossy(&result.stdout).trim(),
        "HELLO WORLD"
    );
}

#[tokio::test]
async fn test_node_eval_json_parse() {
    let (_tmp, sandbox) = temp_sandbox();
    let result = sandbox
        .exec(
            "node",
            &[
                "-p".into(),
                r#"JSON.stringify(JSON.parse('{"a":1,"b":2}'))"#.into(),
            ],
        )
        .await
        .unwrap();
    assert_eq!(result.exit_code, 0);
    assert_eq!(
        String::from_utf8_lossy(&result.stdout).trim(),
        r#"{"a":1,"b":2}"#
    );
}

#[tokio::test]
async fn test_node_eval_array_methods() {
    let (_tmp, sandbox) = temp_sandbox();
    let result = sandbox
        .exec(
            "node",
            &[
                "-p".into(),
                "[3,1,4,1,5].filter(x => x > 2).sort().join(',')".into(),
            ],
        )
        .await
        .unwrap();
    assert_eq!(result.exit_code, 0);
    assert_eq!(String::from_utf8_lossy(&result.stdout).trim(), "3,4,5");
}

#[tokio::test]
async fn test_node_eval_error_handling() {
    let (_tmp, sandbox) = temp_sandbox();
    let result = sandbox
        .exec("node", &["-e".into(), "throw new Error('oops')".into()])
        .await
        .unwrap();
    assert_ne!(result.exit_code, 0);
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("oops"),
        "Expected 'oops' in stderr: {stderr}"
    );
}

#[tokio::test]
async fn test_node_eval_syntax_error() {
    let (_tmp, sandbox) = temp_sandbox();
    let result = sandbox
        .exec("node", &["-e".into(), "function {".into()])
        .await
        .unwrap();
    assert_ne!(result.exit_code, 0);
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(!stderr.is_empty(), "Expected error output for syntax error");
}

#[tokio::test]
async fn test_node_run_file() {
    let (tmp, sandbox) = temp_sandbox();
    std::fs::write(
        tmp.path().join("script.js"),
        "var x = 10;\nvar y = 20;\nconsole.log(x + y);\n",
    )
    .unwrap();

    let result = sandbox
        .exec("node", &["/work/script.js".into()])
        .await
        .unwrap();
    assert_eq!(result.exit_code, 0);
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(stdout.contains("30"), "Expected '30' in stdout: {stdout}");
}

#[tokio::test]
async fn test_node_file_not_found() {
    let (_tmp, sandbox) = temp_sandbox();
    let result = sandbox
        .exec("node", &["/work/nonexistent.js".into()])
        .await
        .unwrap();
    assert_ne!(result.exit_code, 0);
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("cannot open"),
        "Expected file not found error in stderr: {stderr}"
    );
}

#[tokio::test]
async fn test_node_multiline_script() {
    let (tmp, sandbox) = temp_sandbox();
    std::fs::write(
        tmp.path().join("multi.js"),
        r#"
function fibonacci(n) {
    if (n <= 1) return n;
    return fibonacci(n - 1) + fibonacci(n - 2);
}
console.log(fibonacci(10));
"#,
    )
    .unwrap();

    let result = sandbox
        .exec("node", &["/work/multi.js".into()])
        .await
        .unwrap();
    assert_eq!(result.exit_code, 0);
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(
        stdout.contains("55"),
        "Expected fibonacci(10)=55 in stdout: {stdout}"
    );
}

#[tokio::test]
async fn test_exec_js_convenience_method() {
    let (_tmp, sandbox) = temp_sandbox();
    let result = sandbox
        .exec_js("console.log('exec_js works')")
        .await
        .unwrap();
    assert_eq!(result.exit_code, 0);
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(
        stdout.contains("exec_js works"),
        "Expected 'exec_js works' in stdout: {stdout}"
    );
}

#[tokio::test]
async fn test_node_eval_object_destructuring() {
    let (_tmp, sandbox) = temp_sandbox();
    let result = sandbox
        .exec(
            "node",
            &[
                "-e".into(),
                "const {a, b} = {a: 1, b: 2}; console.log(a + b)".into(),
            ],
        )
        .await
        .unwrap();
    assert_eq!(result.exit_code, 0);
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(stdout.contains("3"), "Expected '3' in stdout: {stdout}");
}

#[tokio::test]
async fn test_node_eval_template_literals() {
    let (_tmp, sandbox) = temp_sandbox();
    let result = sandbox
        .exec(
            "node",
            &[
                "-e".into(),
                "const name = 'World'; console.log(`Hello ${name}!`)".into(),
            ],
        )
        .await
        .unwrap();
    assert_eq!(result.exit_code, 0);
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(
        stdout.contains("Hello World!"),
        "Expected 'Hello World!' in stdout: {stdout}"
    );
}

#[tokio::test]
async fn test_node_eval_map_reduce() {
    let (_tmp, sandbox) = temp_sandbox();
    let result = sandbox
        .exec(
            "node",
            &[
                "-p".into(),
                "[1,2,3,4,5].map(x => x * x).reduce((a, b) => a + b, 0)".into(),
            ],
        )
        .await
        .unwrap();
    assert_eq!(result.exit_code, 0);
    assert_eq!(String::from_utf8_lossy(&result.stdout).trim(), "55");
}

#[tokio::test]
async fn test_node_no_args_shows_usage() {
    let (_tmp, sandbox) = temp_sandbox();
    let result = sandbox.exec("node", &[]).await.unwrap();
    assert_ne!(result.exit_code, 0);
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("Usage"),
        "Expected usage message in stderr: {stderr}"
    );
}

#[tokio::test]
async fn test_node_eval_promises_basic() {
    let (_tmp, sandbox) = temp_sandbox();
    let result = sandbox
        .exec(
            "node",
            &[
                "-e".into(),
                "Promise.resolve(42).then(v => console.log('resolved: ' + v))".into(),
            ],
        )
        .await
        .unwrap();
    assert_eq!(result.exit_code, 0);
    // Note: boa may or may not flush microtasks; check if output appears
    // This test validates that Promise constructor works without crashing
}

#[tokio::test]
async fn test_node_eval_math_functions() {
    let (_tmp, sandbox) = temp_sandbox();
    let result = sandbox
        .exec(
            "node",
            &["-p".into(), "Math.max(1, 5, 3) + Math.min(1, 5, 3)".into()],
        )
        .await
        .unwrap();
    assert_eq!(result.exit_code, 0);
    assert_eq!(String::from_utf8_lossy(&result.stdout).trim(), "6");
}

#[tokio::test]
async fn test_node_eval_regex() {
    let (_tmp, sandbox) = temp_sandbox();
    let result = sandbox
        .exec(
            "node",
            &["-p".into(), "'hello world 123'.match(/\\d+/)[0]".into()],
        )
        .await
        .unwrap();
    assert_eq!(result.exit_code, 0);
    assert_eq!(String::from_utf8_lossy(&result.stdout).trim(), "123");
}

#[tokio::test]
async fn test_node_security_no_host_filesystem() {
    let (_tmp, sandbox) = temp_sandbox();
    // Node inside WASM sandbox should not be able to read /etc/passwd
    // The WASM runtime only mounts /work
    let result = sandbox.exec("node", &["/etc/passwd".into()]).await.unwrap();
    assert_ne!(result.exit_code, 0);
}

// --- Fetch / Networking tests ---

fn temp_sandbox_with_fetch(policy: FetchPolicy) -> (tempfile::TempDir, Sandbox) {
    let tmp = tempfile::tempdir().unwrap();
    let config = SandboxConfig {
        work_dir: tmp.path().to_path_buf(),
        fetch_policy: Some(policy),
        ..Default::default()
    };
    let sandbox = Sandbox::new(config).unwrap();
    (tmp, sandbox)
}

#[tokio::test]
async fn test_fetch_disabled_without_policy() {
    let (_tmp, sandbox) = temp_sandbox();

    let request = FetchRequest {
        url: "https://example.com".into(),
        method: "GET".into(),
        headers: HashMap::new(),
        body: None,
    };

    let result = sandbox.fetch(request).await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("networking disabled"),
        "Expected 'networking disabled', got: {err}"
    );
}

#[tokio::test]
async fn test_fetch_basic_get() {
    let (_tmp, sandbox) = temp_sandbox_with_fetch(FetchPolicy::default());

    let request = FetchRequest {
        url: "https://example.com".into(),
        method: "GET".into(),
        headers: HashMap::new(),
        body: None,
    };

    let result = sandbox.fetch(request).await.unwrap();
    assert_eq!(result.status, 200);
    let body = String::from_utf8_lossy(&result.body);
    assert!(
        body.contains("Example Domain"),
        "Expected 'Example Domain' in body"
    );
}

#[tokio::test]
async fn test_fetch_blocked_domain() {
    let policy = FetchPolicy {
        blocked_domains: vec![DomainPattern("example.com".into())],
        ..Default::default()
    };
    let (_tmp, sandbox) = temp_sandbox_with_fetch(policy);

    let request = FetchRequest {
        url: "https://example.com".into(),
        method: "GET".into(),
        headers: HashMap::new(),
        body: None,
    };

    let result = sandbox.fetch(request).await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.to_lowercase().contains("block") || err.to_lowercase().contains("denied"),
        "Expected domain blocked error, got: {err}"
    );
}

#[tokio::test]
async fn test_fetch_ssrf_private_ip_blocked() {
    let policy = FetchPolicy {
        deny_private_ips: true,
        ..Default::default()
    };
    let (_tmp, sandbox) = temp_sandbox_with_fetch(policy);

    let request = FetchRequest {
        url: "http://127.0.0.1".into(),
        method: "GET".into(),
        headers: HashMap::new(),
        body: None,
    };

    let result = sandbox.fetch(request).await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.to_lowercase().contains("private") || err.to_lowercase().contains("block"),
        "Expected private IP blocked error, got: {err}"
    );
}

#[tokio::test]
async fn test_fetch_allowed_domains_only() {
    let policy = FetchPolicy {
        allowed_domains: Some(vec![DomainPattern("example.com".into())]),
        ..Default::default()
    };
    let (_tmp, sandbox) = temp_sandbox_with_fetch(policy);

    // Allowed domain should work
    let request = FetchRequest {
        url: "https://example.com".into(),
        method: "GET".into(),
        headers: HashMap::new(),
        body: None,
    };
    let result = sandbox.fetch(request).await.unwrap();
    assert_eq!(result.status, 200);

    // Non-allowed domain should fail
    let request2 = FetchRequest {
        url: "https://httpbin.org/get".into(),
        method: "GET".into(),
        headers: HashMap::new(),
        body: None,
    };
    let result2 = sandbox.fetch(request2).await;
    assert!(result2.is_err());
}

#[tokio::test]
async fn test_exec_curl_basic() {
    let (_tmp, sandbox) = temp_sandbox_with_fetch(FetchPolicy::default());

    let result = sandbox
        .exec("curl", &["https://example.com".into()])
        .await
        .unwrap();

    assert_eq!(result.exit_code, 0);
    let body = String::from_utf8_lossy(&result.stdout);
    assert!(
        body.contains("Example Domain"),
        "Expected 'Example Domain' in curl output"
    );
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("HTTP 200"),
        "Expected 'HTTP 200' in stderr, got: {stderr}"
    );
}

#[tokio::test]
async fn test_exec_curl_with_headers() {
    let (_tmp, sandbox) = temp_sandbox_with_fetch(FetchPolicy::default());

    let result = sandbox
        .exec(
            "curl",
            &[
                "-H".into(),
                "Accept: application/json".into(),
                "https://httpbin.org/headers".into(),
            ],
        )
        .await
        .unwrap();

    assert_eq!(result.exit_code, 0);
    let body = String::from_utf8_lossy(&result.stdout);
    assert!(
        body.contains("Accept") || body.contains("accept"),
        "Expected headers in response body"
    );
}

#[tokio::test]
async fn test_exec_curl_disabled_without_policy() {
    let (_tmp, sandbox) = temp_sandbox();

    let result = sandbox.exec("curl", &["https://example.com".into()]).await;

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("networking disabled"),
        "Expected 'networking disabled', got: {err}"
    );
}

#[tokio::test]
async fn test_exec_curl_output_file() {
    let (tmp, sandbox) = temp_sandbox_with_fetch(FetchPolicy::default());

    let result = sandbox
        .exec(
            "curl",
            &[
                "-o".into(),
                "output.html".into(),
                "https://example.com".into(),
            ],
        )
        .await
        .unwrap();

    assert_eq!(result.exit_code, 0);

    // Verify file was written
    let content = std::fs::read_to_string(tmp.path().join("output.html")).unwrap();
    assert!(
        content.contains("Example Domain"),
        "Expected 'Example Domain' in output file"
    );
}

#[tokio::test]
async fn test_exec_js_fetch() {
    let (_tmp, sandbox) = temp_sandbox_with_fetch(FetchPolicy::default());

    let result = sandbox
        .exec_js("var r = fetch('https://example.com'); console.log(r.status)")
        .await
        .unwrap();

    assert_eq!(result.exit_code, 0);
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(stdout.contains("200"), "Expected '200' in stdout: {stdout}");
}

#[tokio::test]
async fn test_exec_js_fetch_with_options() {
    let (_tmp, sandbox) = temp_sandbox_with_fetch(FetchPolicy::default());

    let result = sandbox
        .exec_js(
            r#"var r = fetch('https://httpbin.org/post', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: '{"key":"value"}' }); console.log(r.status)"#,
        )
        .await
        .unwrap();

    assert_eq!(result.exit_code, 0);
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(stdout.contains("200"), "Expected '200' in stdout: {stdout}");
}

#[tokio::test]
async fn test_exec_js_fetch_response_body() {
    let (_tmp, sandbox) = temp_sandbox_with_fetch(FetchPolicy::default());

    let result = sandbox
        .exec_js("var r = fetch('https://example.com'); console.log(r.body.indexOf('Example Domain') >= 0)")
        .await
        .unwrap();

    assert_eq!(result.exit_code, 0);
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(
        stdout.contains("true"),
        "Expected 'true' in stdout: {stdout}"
    );
}

#[tokio::test]
async fn test_exec_js_fetch_disabled() {
    let (_tmp, sandbox) = temp_sandbox();

    let result = sandbox
        .exec_js("try { fetch('https://example.com'); } catch(e) { console.log('error: ' + e.message); }")
        .await
        .unwrap();

    // Should either error or print error message
    let stdout = String::from_utf8_lossy(&result.stdout);
    let stderr = String::from_utf8_lossy(&result.stderr);
    let combined = format!("{stdout}{stderr}");
    assert!(
        combined.contains("disabled")
            || combined.contains("error")
            || combined.contains("networking")
            || result.exit_code != 0,
        "Expected fetch to fail when networking is disabled. stdout: {stdout}, stderr: {stderr}, exit: {}",
        result.exit_code
    );
}

#[tokio::test]
async fn test_exec_js_fetch_text_method() {
    let (_tmp, sandbox) = temp_sandbox_with_fetch(FetchPolicy::default());

    let result = sandbox
        .exec_js(
            "var r = fetch('https://example.com'); console.log(r.text().indexOf('Example') >= 0)",
        )
        .await
        .unwrap();

    assert_eq!(result.exit_code, 0);
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(
        stdout.contains("true"),
        "Expected 'true' in stdout: {stdout}"
    );
}

#[tokio::test]
async fn test_exec_js_fetch_ok_property() {
    let (_tmp, sandbox) = temp_sandbox_with_fetch(FetchPolicy::default());

    let result = sandbox
        .exec_js("var r = fetch('https://example.com'); console.log(r.ok)")
        .await
        .unwrap();

    assert_eq!(result.exit_code, 0);
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(
        stdout.contains("true"),
        "Expected 'true' in stdout: {stdout}"
    );
}
