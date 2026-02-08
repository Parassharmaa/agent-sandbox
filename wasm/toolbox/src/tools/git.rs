use std::fs;
use std::path::{Path, PathBuf};

use sha1_smol::Sha1;

/// Minimal git implementation for sandbox use.
/// Supports: init, status, add, commit, log, diff
/// Uses direct file manipulation of .git directory (no external deps).
pub fn run(args: &[String]) -> i32 {
    if args.is_empty() {
        eprintln!("usage: git <command> [<args>]");
        eprintln!("Commands: init, status, add, commit, log, diff");
        return 1;
    }

    match args[0].as_str() {
        "init" => cmd_init(args),
        "status" => cmd_status(args),
        "add" => cmd_add(args),
        "commit" => cmd_commit(args),
        "log" => cmd_log(args),
        "diff" => cmd_diff(args),
        cmd => {
            eprintln!("git: '{}' is not a git command", cmd);
            1
        }
    }
}

fn find_git_dir() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let git_dir = dir.join(".git");
        if git_dir.is_dir() {
            return Some(git_dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn work_tree_from_git_dir(git_dir: &Path) -> PathBuf {
    git_dir.parent().unwrap_or(Path::new(".")).to_path_buf()
}

fn cmd_init(args: &[String]) -> i32 {
    let dir = if args.len() > 1 { &args[1] } else { "." };
    let git_dir = Path::new(dir).join(".git");

    if git_dir.exists() {
        println!(
            "Reinitialized existing Git repository in {}",
            git_dir.display()
        );
        return 0;
    }

    let dirs = ["objects", "refs/heads", "refs/tags"];
    for d in &dirs {
        if let Err(e) = fs::create_dir_all(git_dir.join(d)) {
            eprintln!("git: failed to create {}: {}", d, e);
            return 1;
        }
    }

    // HEAD
    if let Err(e) = fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n") {
        eprintln!("git: failed to write HEAD: {}", e);
        return 1;
    }

    // config
    let config = "[core]\n\trepositoryformatversion = 0\n\tfilemode = true\n\tbare = false\n";
    if let Err(e) = fs::write(git_dir.join("config"), config) {
        eprintln!("git: failed to write config: {}", e);
        return 1;
    }

    println!("Initialized empty Git repository in {}", git_dir.display());
    0
}

fn cmd_status(_args: &[String]) -> i32 {
    let git_dir = match find_git_dir() {
        Some(d) => d,
        None => {
            eprintln!("fatal: not a git repository");
            return 128;
        }
    };

    let work_tree = work_tree_from_git_dir(&git_dir);

    // Read index
    let index = read_index(&git_dir);

    // Get branch name
    let branch = get_current_branch(&git_dir);
    println!("On branch {}", branch);

    // Find tracked, modified, and untracked files
    let mut all_files = Vec::new();
    collect_files(&work_tree, &work_tree, &mut all_files);

    let mut staged: Vec<String> = Vec::new();
    let mut modified: Vec<String> = Vec::new();
    let mut untracked: Vec<String> = Vec::new();

    for file in &all_files {
        if let Some(indexed_hash) = index.get(file) {
            // File is in index, check if modified
            let current_hash = hash_file(&work_tree.join(file));
            if current_hash != *indexed_hash {
                modified.push(file.clone());
            }
            staged.push(file.clone());
        } else {
            untracked.push(file.clone());
        }
    }

    if !staged.is_empty() || !modified.is_empty() || !untracked.is_empty() {
        if !modified.is_empty() {
            println!("\nChanges not staged for commit:");
            for f in &modified {
                println!("\tmodified:   {}", f);
            }
        }

        if !untracked.is_empty() {
            println!("\nUntracked files:");
            for f in &untracked {
                println!("\t{}", f);
            }
        }
    } else {
        println!("nothing to commit, working tree clean");
    }

    0
}

fn cmd_add(args: &[String]) -> i32 {
    let git_dir = match find_git_dir() {
        Some(d) => d,
        None => {
            eprintln!("fatal: not a git repository");
            return 128;
        }
    };

    let work_tree = work_tree_from_git_dir(&git_dir);

    if args.len() < 2 {
        eprintln!("Nothing specified, nothing added.");
        return 0;
    }

    let mut index = read_index(&git_dir);

    for arg in &args[1..] {
        if arg == "." || arg == "-A" {
            // Add all files
            let mut all_files = Vec::new();
            collect_files(&work_tree, &work_tree, &mut all_files);
            for file in all_files {
                let hash = hash_file(&work_tree.join(&file));
                // Store object
                store_object(&git_dir, &hash, &work_tree.join(&file));
                index.insert(file, hash);
            }
        } else {
            let path = work_tree.join(arg);
            if path.is_file() {
                let rel = arg.to_string();
                let hash = hash_file(&path);
                store_object(&git_dir, &hash, &path);
                index.insert(rel, hash);
            } else if path.is_dir() {
                let mut files = Vec::new();
                collect_files(&path, &work_tree, &mut files);
                for file in files {
                    let hash = hash_file(&work_tree.join(&file));
                    store_object(&git_dir, &hash, &work_tree.join(&file));
                    index.insert(file, hash);
                }
            } else {
                eprintln!("fatal: pathspec '{}' did not match any files", arg);
                return 128;
            }
        }
    }

    write_index(&git_dir, &index);
    0
}

fn cmd_commit(args: &[String]) -> i32 {
    let git_dir = match find_git_dir() {
        Some(d) => d,
        None => {
            eprintln!("fatal: not a git repository");
            return 128;
        }
    };

    let index = read_index(&git_dir);
    if index.is_empty() {
        eprintln!("nothing to commit");
        return 1;
    }

    // Get message
    let mut message = String::new();
    let mut i = 1;
    while i < args.len() {
        if args[i] == "-m" && i + 1 < args.len() {
            message = args[i + 1].clone();
            i += 2;
        } else {
            i += 1;
        }
    }

    if message.is_empty() {
        eprintln!("error: no commit message given");
        return 1;
    }

    // Build tree object (simplified: flat list)
    let mut tree_content = String::new();
    let mut sorted_entries: Vec<_> = index.iter().collect();
    sorted_entries.sort_by(|(a, _), (b, _)| a.cmp(b));
    for (name, hash) in &sorted_entries {
        tree_content.push_str(&format!("100644 blob {} {}\n", hash, name));
    }
    let tree_hash = sha1_hash(tree_content.as_bytes());

    // Get parent commit
    let parent = get_head_commit(&git_dir);

    // Build commit object
    let mut commit_content = format!("tree {}\n", tree_hash);
    if let Some(parent_hash) = &parent {
        commit_content.push_str(&format!("parent {}\n", parent_hash));
    }
    commit_content.push_str(&format!(
        "author Sandbox User <sandbox@example.com> 0 +0000\n\
         committer Sandbox User <sandbox@example.com> 0 +0000\n\n\
         {}\n",
        message
    ));
    let commit_hash = sha1_hash(commit_content.as_bytes());

    // Store tree and commit objects
    store_raw_object(&git_dir, &tree_hash, tree_content.as_bytes());
    store_raw_object(&git_dir, &commit_hash, commit_content.as_bytes());

    // Update HEAD ref
    let branch = get_current_branch(&git_dir);
    let ref_path = git_dir.join("refs/heads").join(&branch);
    if let Some(parent_dir) = ref_path.parent() {
        let _ = fs::create_dir_all(parent_dir);
    }
    let _ = fs::write(&ref_path, format!("{}\n", commit_hash));

    let short_hash = &commit_hash[..7];
    println!("[{} {}] {}", branch, short_hash, message);
    println!(" {} file(s) changed", index.len());

    0
}

fn cmd_log(_args: &[String]) -> i32 {
    let git_dir = match find_git_dir() {
        Some(d) => d,
        None => {
            eprintln!("fatal: not a git repository");
            return 128;
        }
    };

    let mut current = get_head_commit(&git_dir);
    let mut count = 0;

    while let Some(hash) = current {
        if count > 50 {
            break; // Safety limit
        }

        let obj = match read_raw_object(&git_dir, &hash) {
            Some(o) => o,
            None => break,
        };

        let content = String::from_utf8_lossy(&obj);
        println!("commit {}", hash);

        for line in content.lines() {
            if let Some(author) = line.strip_prefix("author ") {
                println!("Author: {}", author);
            }
        }

        // Find commit message (after blank line)
        if let Some(msg_start) = content.find("\n\n") {
            let msg = content[msg_start + 2..].trim();
            println!("\n    {}\n", msg);
        }

        // Find parent
        current = None;
        for line in content.lines() {
            if let Some(parent_hash) = line.strip_prefix("parent ") {
                current = Some(parent_hash.trim().to_string());
                break;
            }
        }

        count += 1;
    }

    if count == 0 {
        eprintln!("fatal: no commits yet");
        return 128;
    }

    0
}

fn cmd_diff(_args: &[String]) -> i32 {
    let git_dir = match find_git_dir() {
        Some(d) => d,
        None => {
            eprintln!("fatal: not a git repository");
            return 128;
        }
    };

    let work_tree = work_tree_from_git_dir(&git_dir);
    let index = read_index(&git_dir);

    for (file, indexed_hash) in &index {
        let path = work_tree.join(file);
        if !path.exists() {
            println!("diff --git a/{} b/{}", file, file);
            println!("deleted file");
            continue;
        }

        let current_hash = hash_file(&path);
        if current_hash != *indexed_hash {
            println!("diff --git a/{} b/{}", file, file);
            // Show simple diff using similar crate if available,
            // otherwise just note the change
            if let Ok(current_content) = fs::read_to_string(&path) {
                // Read old content from object store
                if let Some(old_content_bytes) = read_raw_object(&git_dir, indexed_hash) {
                    let old_content = String::from_utf8_lossy(&old_content_bytes);
                    print_unified_diff(file, &old_content, &current_content);
                } else {
                    println!("Binary files differ");
                }
            }
        }
    }

    0
}

fn print_unified_diff(filename: &str, old: &str, new: &str) {
    println!("--- a/{}", filename);
    println!("+++ b/{}", filename);

    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    // Simple line-by-line diff
    let max = old_lines.len().max(new_lines.len());
    let mut in_hunk = false;

    for i in 0..max {
        let old_line = old_lines.get(i).copied();
        let new_line = new_lines.get(i).copied();

        match (old_line, new_line) {
            (Some(o), Some(n)) if o == n => {
                if in_hunk {
                    println!(" {}", o);
                }
            }
            (Some(o), Some(n)) => {
                if !in_hunk {
                    println!(
                        "@@ -{},{} +{},{} @@",
                        i + 1,
                        old_lines.len() - i,
                        i + 1,
                        new_lines.len() - i
                    );
                    in_hunk = true;
                }
                println!("-{}", o);
                println!("+{}", n);
            }
            (Some(o), None) => {
                if !in_hunk {
                    println!("@@ -{},{} +{},{} @@", i + 1, old_lines.len() - i, 0, 0);
                    in_hunk = true;
                }
                println!("-{}", o);
            }
            (None, Some(n)) => {
                if !in_hunk {
                    println!("@@ -0,0 +{},{} @@", i + 1, new_lines.len() - i);
                    in_hunk = true;
                }
                println!("+{}", n);
            }
            (None, None) => break,
        }
    }
}

// --- Helper functions ---

type Index = std::collections::HashMap<String, String>;

fn read_index(git_dir: &Path) -> Index {
    let index_path = git_dir.join("index.txt"); // Simplified text-based index
    let mut index = Index::new();
    if let Ok(content) = fs::read_to_string(&index_path) {
        for line in content.lines() {
            if let Some((hash, name)) = line.split_once(' ') {
                index.insert(name.to_string(), hash.to_string());
            }
        }
    }
    index
}

fn write_index(git_dir: &Path, index: &Index) {
    let index_path = git_dir.join("index.txt");
    let mut content = String::new();
    let mut entries: Vec<_> = index.iter().collect();
    entries.sort_by(|(a, _), (b, _)| a.cmp(b));
    for (name, hash) in entries {
        content.push_str(&format!("{} {}\n", hash, name));
    }
    let _ = fs::write(index_path, content);
}

fn get_current_branch(git_dir: &Path) -> String {
    let head = fs::read_to_string(git_dir.join("HEAD")).unwrap_or_default();
    if head.starts_with("ref: refs/heads/") {
        head.trim()
            .strip_prefix("ref: refs/heads/")
            .unwrap_or("main")
            .to_string()
    } else {
        "main".to_string()
    }
}

fn get_head_commit(git_dir: &Path) -> Option<String> {
    let head = fs::read_to_string(git_dir.join("HEAD")).ok()?;
    let head = head.trim();
    if head.starts_with("ref: ") {
        let ref_path = git_dir.join(head.strip_prefix("ref: ")?);
        fs::read_to_string(ref_path)
            .ok()
            .map(|s| s.trim().to_string())
    } else {
        Some(head.to_string())
    }
}

fn hash_file(path: &Path) -> String {
    let data = fs::read(path).unwrap_or_default();
    sha1_hash(&data)
}

fn sha1_hash(data: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(data);
    hasher.digest().to_string()
}

fn store_object(git_dir: &Path, hash: &str, path: &Path) {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(_) => return,
    };
    store_raw_object(git_dir, hash, &data);
}

fn store_raw_object(git_dir: &Path, hash: &str, data: &[u8]) {
    let obj_dir = git_dir.join("objects").join(&hash[..2]);
    let _ = fs::create_dir_all(&obj_dir);
    let obj_path = obj_dir.join(&hash[2..]);
    let _ = fs::write(obj_path, data);
}

fn read_raw_object(git_dir: &Path, hash: &str) -> Option<Vec<u8>> {
    if hash.len() < 3 {
        return None;
    }
    let obj_path = git_dir.join("objects").join(&hash[..2]).join(&hash[2..]);
    fs::read(obj_path).ok()
}

fn collect_files(dir: &Path, root: &Path, files: &mut Vec<String>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Skip .git directory
        if name == ".git" {
            continue;
        }

        if path.is_dir() {
            collect_files(&path, root, files);
        } else if let Ok(rel) = path.strip_prefix(root) {
            files.push(rel.to_string_lossy().to_string());
        }
    }
}
