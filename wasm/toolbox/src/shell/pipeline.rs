use std::fs;
use std::sync::atomic::{AtomicU32, Ordering};

static PIPE_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Get a unique temp file path for pipe buffers.
pub fn pipe_temp_file() -> String {
    let n = PIPE_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("/work/.sh_pipe_{}", n)
}

/// Clean up a temp pipe file.
pub fn cleanup_pipe(path: &str) {
    let _ = fs::remove_file(path);
}

/// Execute a pipeline: run each stage sequentially, using temp files as pipe buffers.
/// Returns the exit code of the last command in the pipeline.
///
/// The actual execution is done by the caller — this module just manages the temp files.
pub struct PipelineState {
    /// Temp files used as pipe buffers between stages.
    pub pipe_files: Vec<String>,
}

impl PipelineState {
    /// Create state for a pipeline with `n` commands.
    pub fn new(num_commands: usize) -> Self {
        let mut pipe_files = Vec::new();
        // We need n-1 pipe files for n commands
        for _ in 0..num_commands.saturating_sub(1) {
            pipe_files.push(pipe_temp_file());
        }
        PipelineState { pipe_files }
    }

    /// Get the input file for stage `i` (None for the first stage).
    pub fn input_for(&self, stage: usize) -> Option<&str> {
        if stage == 0 {
            None
        } else {
            self.pipe_files.get(stage - 1).map(|s| s.as_str())
        }
    }

    /// Get the output file for stage `i` (None for the last stage).
    pub fn output_for(&self, stage: usize) -> Option<&str> {
        self.pipe_files.get(stage).map(|s| s.as_str())
    }

    /// Clean up all pipe temp files.
    pub fn cleanup(&self) {
        for f in &self.pipe_files {
            cleanup_pipe(f);
        }
    }
}
