use std::time::Instant;

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

/// Measure first sandbox creation (includes WASM module compilation).
#[tokio::test]
async fn perf_first_sandbox_creation() {
    let tmp = tempfile::tempdir().unwrap();
    let config = SandboxConfig {
        work_dir: tmp.path().to_path_buf(),
        ..Default::default()
    };

    let start = Instant::now();
    let sandbox = Sandbox::new(config).unwrap();
    let elapsed = start.elapsed();

    println!("[perf] first sandbox creation: {:?}", elapsed);
    sandbox.destroy().await.unwrap();
}

/// Measure subsequent sandbox creation (module already cached).
#[tokio::test]
async fn perf_sandbox_creation_cached() {
    // Ensure module is compiled first
    let (_tmp, sandbox) = temp_sandbox();
    sandbox.destroy().await.unwrap();

    let mut times = Vec::new();
    for _ in 0..10 {
        let tmp = tempfile::tempdir().unwrap();
        let config = SandboxConfig {
            work_dir: tmp.path().to_path_buf(),
            ..Default::default()
        };

        let start = Instant::now();
        let sandbox = Sandbox::new(config).unwrap();
        times.push(start.elapsed());
        sandbox.destroy().await.unwrap();
    }

    let avg = times.iter().sum::<std::time::Duration>() / times.len() as u32;
    println!("[perf] cached sandbox creation (10 runs): avg {:?}", avg);
}

/// Measure simple command execution (echo).
#[tokio::test]
async fn perf_exec_echo() {
    let (_tmp, sandbox) = temp_sandbox();

    // Warmup
    sandbox.exec("echo", &["warmup".to_string()]).await.unwrap();

    let mut times = Vec::new();
    for i in 0..20 {
        let start = Instant::now();
        sandbox
            .exec("echo", &[format!("iteration {i}")])
            .await
            .unwrap();
        times.push(start.elapsed());
    }

    let avg = times.iter().sum::<std::time::Duration>() / times.len() as u32;
    let min = times.iter().min().unwrap();
    let max = times.iter().max().unwrap();
    println!(
        "[perf] exec echo (20 runs): avg {:?}, min {:?}, max {:?}",
        avg, min, max
    );
}

/// Measure file-heavy command (find).
#[tokio::test]
async fn perf_exec_find() {
    let (_tmp, sandbox) = temp_sandbox();

    // Create some files
    for i in 0..50 {
        sandbox
            .write_file(
                &format!("dir/file_{i}.txt"),
                format!("content {i}").as_bytes(),
            )
            .await
            .unwrap();
    }

    let mut times = Vec::new();
    for _ in 0..10 {
        let start = Instant::now();
        sandbox
            .exec("find", &["/work/dir".to_string()])
            .await
            .unwrap();
        times.push(start.elapsed());
    }

    let avg = times.iter().sum::<std::time::Duration>() / times.len() as u32;
    println!("[perf] exec find (50 files, 10 runs): avg {:?}", avg);
}

/// Measure grep over files.
#[tokio::test]
async fn perf_exec_grep() {
    let (_tmp, sandbox) = temp_sandbox();

    // Create files with content
    for i in 0..20 {
        let content = format!("line 1\nTODO: fix item {i}\nline 3\nanother TODO here\nline 5\n");
        sandbox
            .write_file(&format!("src/file_{i}.rs"), content.as_bytes())
            .await
            .unwrap();
    }

    let mut times = Vec::new();
    for _ in 0..10 {
        let start = Instant::now();
        sandbox
            .exec(
                "grep",
                &[
                    "-r".to_string(),
                    "TODO".to_string(),
                    "/work/src".to_string(),
                ],
            )
            .await
            .unwrap();
        times.push(start.elapsed());
    }

    let avg = times.iter().sum::<std::time::Duration>() / times.len() as u32;
    println!(
        "[perf] exec grep -r TODO (20 files, 10 runs): avg {:?}",
        avg
    );
}

/// Measure sed execution.
#[tokio::test]
async fn perf_exec_sed() {
    let (_tmp, sandbox) = temp_sandbox();

    sandbox
        .write_file("data.txt", b"hello world\nhello rust\nhello wasm\n")
        .await
        .unwrap();

    let mut times = Vec::new();
    for _ in 0..10 {
        let start = Instant::now();
        sandbox
            .exec(
                "sed",
                &[
                    "s/hello/goodbye/g".to_string(),
                    "/work/data.txt".to_string(),
                ],
            )
            .await
            .unwrap();
        times.push(start.elapsed());
    }

    let avg = times.iter().sum::<std::time::Duration>() / times.len() as u32;
    println!("[perf] exec sed (10 runs): avg {:?}", avg);
}

/// Measure write_file + read_file round trip.
#[tokio::test]
async fn perf_file_io_roundtrip() {
    let (_tmp, sandbox) = temp_sandbox();
    let data = vec![b'x'; 1024 * 100]; // 100KB

    let mut times = Vec::new();
    for i in 0..20 {
        let start = Instant::now();
        sandbox
            .write_file(&format!("perf_{i}.bin"), &data)
            .await
            .unwrap();
        let _ = sandbox.read_file(&format!("perf_{i}.bin")).await.unwrap();
        times.push(start.elapsed());
    }

    let avg = times.iter().sum::<std::time::Duration>() / times.len() as u32;
    println!("[perf] file I/O roundtrip 100KB (20 runs): avg {:?}", avg);
}

/// Measure diff with many file changes.
#[tokio::test]
async fn perf_diff() {
    let (_tmp, sandbox) = temp_sandbox();

    // Create changes
    for i in 0..50 {
        sandbox
            .write_file(&format!("new_{i}.txt"), format!("content {i}").as_bytes())
            .await
            .unwrap();
    }

    let mut times = Vec::new();
    for _ in 0..10 {
        let start = Instant::now();
        let changes = sandbox.diff().await.unwrap();
        times.push(start.elapsed());
        assert_eq!(changes.len(), 50);
    }

    let avg = times.iter().sum::<std::time::Duration>() / times.len() as u32;
    println!("[perf] diff (50 new files, 10 runs): avg {:?}", avg);
}

/// Measure sequential command throughput.
#[tokio::test]
async fn perf_sequential_commands() {
    let (_tmp, sandbox) = temp_sandbox();

    sandbox
        .write_file("test.txt", b"hello world\n")
        .await
        .unwrap();

    let commands: Vec<(&str, Vec<String>)> = vec![
        ("cat", vec!["/work/test.txt".to_string()]),
        ("wc", vec!["-l".to_string(), "/work/test.txt".to_string()]),
        (
            "head",
            vec![
                "-n".to_string(),
                "1".to_string(),
                "/work/test.txt".to_string(),
            ],
        ),
        ("echo", vec!["done".to_string()]),
    ];

    let start = Instant::now();
    let iterations = 10;
    for _ in 0..iterations {
        for (cmd, args) in &commands {
            sandbox.exec(cmd, args).await.unwrap();
        }
    }
    let elapsed = start.elapsed();
    let total_cmds = iterations * commands.len();

    println!(
        "[perf] {} sequential commands in {:?} ({:?}/cmd)",
        total_cmds,
        elapsed,
        elapsed / total_cmds as u32
    );
}
