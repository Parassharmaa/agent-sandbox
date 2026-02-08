/// Available tools in the WASM toolbox.
pub const AVAILABLE_TOOLS: &[&str] = &[
    // File viewing
    "cat",
    "head",
    "tail",
    "touch",
    "tee",
    // Search
    "grep",
    "rg",
    "find",
    "tree",
    // Text processing
    "sed",
    "sort",
    "uniq",
    "cut",
    "tr",
    "wc",
    // Data
    "jq",
    "diff",
    "patch",
    "base64",
    "sha256sum",
    "xxd",
    // File management
    "ls",
    "mkdir",
    "cp",
    "mv",
    "rm",
    "du",
    "ln",
    "stat",
    // Archive
    "tar",
    "gzip",
    "zip",
    // Version control
    "git",
    // JavaScript runtime
    "node",
    // Networking
    "curl",
    // Shell utils
    "echo",
    "printf",
    "env",
    "xargs",
    "basename",
    "dirname",
];

/// Check if a command is available in the toolbox.
pub fn is_available(command: &str) -> bool {
    AVAILABLE_TOOLS.contains(&command)
}
