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
    "awk",
    "tac",
    "rev",
    "nl",
    "paste",
    "comm",
    "join",
    "fold",
    "column",
    "expand",
    "unexpand",
    "strings",
    "od",
    // Data / hashing
    "jq",
    "diff",
    "patch",
    "base64",
    "sha256sum",
    "sha1sum",
    "md5sum",
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
    "readlink",
    "rmdir",
    "split",
    "file",
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
    "seq",
    "sleep",
    "which",
    "whoami",
    "hostname",
    "printenv",
    "date",
    "expr",
    "true",
    "false",
    "test",
    "[",
    // Shell interpreter
    "sh",
    "bash",
];

/// Check if a command is available in the toolbox.
pub fn is_available(command: &str) -> bool {
    AVAILABLE_TOOLS.contains(&command)
}
