use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Tracking allocator that wraps System and counts current heap usage.
struct TrackingAllocator;

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            let current = ALLOCATED.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
            PEAK.fetch_max(current, Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        ALLOCATED.fetch_sub(layout.size(), Ordering::Relaxed);
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = unsafe { System.realloc(ptr, layout, new_size) };
        if !new_ptr.is_null() {
            if new_size > layout.size() {
                let delta = new_size - layout.size();
                let current = ALLOCATED.fetch_add(delta, Ordering::Relaxed) + delta;
                PEAK.fetch_max(current, Ordering::Relaxed);
            } else {
                ALLOCATED.fetch_sub(layout.size() - new_size, Ordering::Relaxed);
            }
        }
        new_ptr
    }
}

#[global_allocator]
static GLOBAL: TrackingAllocator = TrackingAllocator;

fn current_allocated() -> usize {
    ALLOCATED.load(Ordering::Relaxed)
}

fn reset_peak() {
    PEAK.store(ALLOCATED.load(Ordering::Relaxed), Ordering::Relaxed);
}

fn peak_allocated() -> usize {
    PEAK.load(Ordering::Relaxed)
}

fn format_bytes(bytes: usize) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

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

/// Memory cost of first sandbox creation (module deserialization + setup).
#[tokio::test]
async fn mem_first_sandbox_creation() {
    let before = current_allocated();
    reset_peak();

    let tmp = tempfile::tempdir().unwrap();
    let config = SandboxConfig {
        work_dir: tmp.path().to_path_buf(),
        ..Default::default()
    };
    let sandbox = Sandbox::new(config).unwrap();

    let after = current_allocated();
    let peak = peak_allocated();

    println!(
        "[mem] first sandbox creation: heap delta = {}, peak = {}, baseline = {}",
        format_bytes(after.saturating_sub(before)),
        format_bytes(peak),
        format_bytes(before),
    );

    sandbox.destroy().await.unwrap();
    let after_destroy = current_allocated();
    println!(
        "[mem] after destroy: heap = {} (freed {})",
        format_bytes(after_destroy),
        format_bytes(after.saturating_sub(after_destroy)),
    );
}

/// Memory cost of additional sandbox instances (module already cached).
#[tokio::test]
async fn mem_additional_sandboxes() {
    // First sandbox triggers module deserialization
    let (_tmp0, sandbox0) = temp_sandbox();

    let before = current_allocated();

    let mut sandboxes = Vec::new();
    let mut tmps = Vec::new();
    for _ in 0..10 {
        let tmp = tempfile::tempdir().unwrap();
        let config = SandboxConfig {
            work_dir: tmp.path().to_path_buf(),
            ..Default::default()
        };
        let sandbox = Sandbox::new(config).unwrap();
        sandboxes.push(sandbox);
        tmps.push(tmp);
    }

    let after = current_allocated();
    let per_sandbox = after.saturating_sub(before) / 10;

    println!(
        "[mem] 10 additional sandboxes: total delta = {}, per sandbox = {}",
        format_bytes(after.saturating_sub(before)),
        format_bytes(per_sandbox),
    );

    // Destroy all and measure cleanup
    for s in &sandboxes {
        s.destroy().await.unwrap();
    }
    drop(sandboxes);
    drop(tmps);

    let after_cleanup = current_allocated();
    println!(
        "[mem] after destroying all 10: freed {}",
        format_bytes(after.saturating_sub(after_cleanup)),
    );

    sandbox0.destroy().await.unwrap();
}

/// Memory usage per exec call — checks for leaks.
#[tokio::test]
async fn mem_exec_leak_check() {
    let (_tmp, sandbox) = temp_sandbox();

    // Warmup
    sandbox.exec("echo", &["warmup".to_string()]).await.unwrap();

    let before = current_allocated();

    for i in 0..100 {
        sandbox
            .exec("echo", &[format!("iteration {i}")])
            .await
            .unwrap();
    }

    let after = current_allocated();
    let delta = after as isize - before as isize;
    let per_exec = delta / 100;

    println!(
        "[mem] 100 exec calls: heap delta = {} bytes ({} bytes/exec)",
        delta, per_exec,
    );

    // A small positive delta is OK (allocator fragmentation), but large growth
    // indicates a leak. Flag if >1KB per exec.
    if per_exec > 1024 {
        println!(
            "[mem] WARNING: possible memory leak — {} bytes/exec",
            per_exec
        );
    }
}

/// Peak memory during a heavy grep operation.
#[tokio::test]
async fn mem_peak_during_heavy_exec() {
    let (_tmp, sandbox) = temp_sandbox();

    // Create many files with content
    for i in 0..100 {
        let content = format!(
            "{}\n",
            "The quick brown fox jumps over the lazy dog. ".repeat(100)
        );
        sandbox
            .write_file(&format!("data/file_{i}.txt"), content.as_bytes())
            .await
            .unwrap();
    }

    let before = current_allocated();
    reset_peak();

    sandbox
        .exec(
            "grep",
            &[
                "-r".to_string(),
                "fox".to_string(),
                "/work/data".to_string(),
            ],
        )
        .await
        .unwrap();

    let after = current_allocated();
    let peak = peak_allocated();

    println!(
        "[mem] grep 100 files: before = {}, after = {}, peak = {}, peak delta = {}",
        format_bytes(before),
        format_bytes(after),
        format_bytes(peak),
        format_bytes(peak.saturating_sub(before)),
    );
}

/// Memory used by stdout/stderr pipe buffers.
#[tokio::test]
async fn mem_large_output() {
    let (_tmp, sandbox) = temp_sandbox();

    // Generate large output
    sandbox
        .write_file("big.txt", "x\n".repeat(10_000).as_bytes())
        .await
        .unwrap();

    let before = current_allocated();
    reset_peak();

    let result = sandbox
        .exec("cat", &["/work/big.txt".to_string()])
        .await
        .unwrap();

    let after = current_allocated();
    let peak = peak_allocated();

    println!(
        "[mem] cat 20KB file: output size = {}, heap delta = {}, peak delta = {}",
        format_bytes(result.stdout.len()),
        format_bytes(after.saturating_sub(before)),
        format_bytes(peak.saturating_sub(before)),
    );
}

/// Memory with diff on many files.
#[tokio::test]
async fn mem_diff_many_files() {
    let (_tmp, sandbox) = temp_sandbox();

    for i in 0..200 {
        sandbox
            .write_file(
                &format!("files/f_{i}.txt"),
                format!("content {i}").as_bytes(),
            )
            .await
            .unwrap();
    }

    let before = current_allocated();
    reset_peak();

    let changes = sandbox.diff().await.unwrap();
    assert_eq!(changes.len(), 200);

    let after = current_allocated();
    let peak = peak_allocated();

    println!(
        "[mem] diff 200 files: heap delta = {}, peak delta = {}",
        format_bytes(after.saturating_sub(before)),
        format_bytes(peak.saturating_sub(before)),
    );
}
