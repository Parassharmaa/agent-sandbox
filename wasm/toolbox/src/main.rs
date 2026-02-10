pub mod fetch;
mod shell;
mod tools;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    // BusyBox-style dispatch: check argv[0] or TOOLBOX_CMD env var
    let cmd = env::var("TOOLBOX_CMD")
        .ok()
        .or_else(|| {
            args.first().and_then(|a| {
                let name = std::path::Path::new(a)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(a);
                if name == "toolbox" {
                    None
                } else {
                    Some(name.to_string())
                }
            })
        })
        .unwrap_or_default();

    // If invoked as "toolbox <cmd> [args...]", shift args
    let (cmd, tool_args) = if cmd.is_empty() {
        if args.len() < 2 {
            eprintln!("Usage: toolbox <command> [args...]");
            eprintln!("Available commands:");
            print_available_commands();
            std::process::exit(1);
        }
        (args[1].clone(), &args[2..])
    } else {
        (cmd, &args[1..])
    };

    let tool_args: Vec<String> = tool_args.to_vec();
    let exit_code = dispatch(&cmd, &tool_args);
    std::process::exit(exit_code);
}

pub fn dispatch(cmd: &str, args: &[String]) -> i32 {
    match cmd {
        // File viewing
        "cat" => tools::cat::run(args),
        "head" => tools::head::run(args),
        "tail" => tools::tail::run(args),
        "touch" => tools::touch::run(args),
        "tee" => tools::tee::run(args),

        // Search
        "grep" => tools::grep::run(args),
        "rg" => tools::rg::run(args),
        "find" => tools::find::run(args),
        "tree" => tools::tree::run(args),

        // Text processing
        "sed" => tools::sed::run(args),
        "sort" => tools::sort::run(args),
        "uniq" => tools::uniq::run(args),
        "cut" => tools::cut::run(args),
        "tr" => tools::tr::run(args),
        "wc" => tools::wc::run(args),
        "awk" => tools::awk::run(args),
        "tac" => tools::tac::run(args),
        "rev" => tools::rev::run(args),
        "nl" => tools::nl::run(args),
        "paste" => tools::paste::run(args),
        "comm" => tools::comm::run(args),
        "join" => tools::join::run(args),
        "fold" => tools::fold::run(args),
        "column" => tools::column::run(args),
        "expand" => tools::expand_tool::run(args),
        "unexpand" => tools::unexpand::run(args),
        "strings" => tools::strings::run(args),
        "od" => tools::od::run(args),

        // Data / hashing
        "jq" => tools::jq::run(args),
        "diff" => tools::diff::run(args),
        "patch" => tools::patch::run(args),
        "base64" => tools::base64_tool::run(args),
        "sha256sum" => tools::sha256sum::run(args),
        "sha1sum" => tools::sha1sum::run(args),
        "md5sum" => tools::md5sum::run(args),
        "xxd" => tools::xxd::run(args),

        // File management
        "ls" => tools::ls::run(args),
        "mkdir" => tools::mkdir::run(args),
        "cp" => tools::cp::run(args),
        "mv" => tools::mv::run(args),
        "rm" => tools::rm::run(args),
        "du" => tools::du::run(args),
        "ln" => tools::ln::run(args),
        "stat" => tools::stat::run(args),
        "readlink" => tools::readlink::run(args),
        "rmdir" => tools::rmdir::run(args),
        "split" => tools::split::run(args),
        "file" => tools::file::run(args),

        // Archive
        "tar" => tools::tar_tool::run(args),
        "gzip" => tools::gzip::run(args),
        "zip" => tools::zip::run(args),

        // Version control
        "git" => tools::git::run(args),

        // JavaScript runtime
        "node" => tools::node::run(args),

        // Shell utils
        "echo" => tools::echo::run(args),
        "printf" => tools::printf::run(args),
        "env" => tools::env::run(args),
        "xargs" => tools::xargs::run(args),
        "basename" => tools::basename::run(args),
        "dirname" => tools::dirname::run(args),
        "seq" => tools::seq::run(args),
        "sleep" => tools::sleep::run(args),
        "which" => tools::which::run(args),
        "whoami" => tools::whoami::run(args),
        "hostname" => tools::hostname::run(args),
        "printenv" => tools::printenv::run(args),
        "date" => tools::date::run(args),
        "expr" => tools::expr::run(args),
        "true" => 0,
        "false" => 1,
        "test" | "[" => {
            let args_vec: Vec<String> = args.to_vec();
            let args_ref: Vec<&str> = args_vec.iter().map(|s| s.as_str()).collect();
            let args_ref = if !args_ref.is_empty() && *args_ref.last().unwrap() == "]" {
                &args_ref[..args_ref.len() - 1]
            } else {
                &args_ref
            };
            if eval_test_args(args_ref) { 0 } else { 1 }
        }

        // Shell interpreter
        "sh" | "bash" => shell::run(args, dispatch),

        _ => {
            eprintln!("toolbox: unknown command '{cmd}'");
            eprintln!("Available commands:");
            print_available_commands();
            1
        }
    }
}

fn eval_test_args(args: &[&str]) -> bool {
    match args.len() {
        0 => false,
        1 => !args[0].is_empty(),
        2 => match args[0] {
            "-z" => args[1].is_empty(),
            "-n" => !args[1].is_empty(),
            "!" => !eval_test_args(&args[1..]),
            "-e" | "-f" => std::path::Path::new(args[1]).exists(),
            "-d" => std::path::Path::new(args[1]).is_dir(),
            _ => !args[0].is_empty(),
        },
        _ => {
            if args.len() >= 3 {
                let left = args[0];
                let op = args[1];
                let right = args[2];
                match op {
                    "=" | "==" => left == right,
                    "!=" => left != right,
                    "-eq" => left.parse::<i64>().unwrap_or(0) == right.parse::<i64>().unwrap_or(0),
                    "-ne" => left.parse::<i64>().unwrap_or(0) != right.parse::<i64>().unwrap_or(0),
                    "-lt" => left.parse::<i64>().unwrap_or(0) < right.parse::<i64>().unwrap_or(0),
                    "-le" => left.parse::<i64>().unwrap_or(0) <= right.parse::<i64>().unwrap_or(0),
                    "-gt" => left.parse::<i64>().unwrap_or(0) > right.parse::<i64>().unwrap_or(0),
                    "-ge" => left.parse::<i64>().unwrap_or(0) >= right.parse::<i64>().unwrap_or(0),
                    _ => false,
                }
            } else {
                false
            }
        }
    }
}

fn print_available_commands() {
    let commands = [
        "cat", "head", "tail", "touch", "tee",
        "grep", "rg", "find", "tree",
        "sed", "sort", "uniq", "cut", "tr", "wc",
        "awk", "tac", "rev", "nl", "paste", "comm", "join", "fold", "column",
        "expand", "unexpand", "strings", "od",
        "jq", "diff", "patch", "base64", "sha256sum", "sha1sum", "md5sum", "xxd",
        "ls", "mkdir", "cp", "mv", "rm", "du", "ln", "stat",
        "readlink", "rmdir", "split", "file",
        "tar", "gzip", "zip",
        "git", "node",
        "echo", "printf", "env", "xargs", "basename", "dirname",
        "seq", "sleep", "which", "whoami", "hostname", "printenv", "date", "expr",
        "true", "false", "test", "[",
        "sh", "bash",
    ];
    for cmd in commands {
        eprintln!("  {cmd}");
    }
}
