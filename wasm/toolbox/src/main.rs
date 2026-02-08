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

fn dispatch(cmd: &str, args: &[String]) -> i32 {
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

        // Data
        "jq" => tools::jq::run(args),
        "diff" => tools::diff::run(args),
        "patch" => tools::patch::run(args),
        "base64" => tools::base64_tool::run(args),
        "sha256sum" => tools::sha256sum::run(args),
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

        // Archive
        "tar" => tools::tar_tool::run(args),
        "gzip" => tools::gzip::run(args),
        "zip" => tools::zip::run(args),

        // Version control
        "git" => tools::git::run(args),

        // Shell utils
        "echo" => tools::echo::run(args),
        "printf" => tools::printf::run(args),
        "env" => tools::env::run(args),
        "xargs" => tools::xargs::run(args),
        "basename" => tools::basename::run(args),
        "dirname" => tools::dirname::run(args),

        _ => {
            eprintln!("toolbox: unknown command '{cmd}'");
            eprintln!("Available commands:");
            print_available_commands();
            1
        }
    }
}

fn print_available_commands() {
    let commands = [
        "cat",
        "head",
        "tail",
        "touch",
        "tee",
        "grep",
        "rg",
        "find",
        "tree",
        "sed",
        "sort",
        "uniq",
        "cut",
        "tr",
        "wc",
        "jq",
        "diff",
        "patch",
        "base64",
        "sha256sum",
        "xxd",
        "ls",
        "mkdir",
        "cp",
        "mv",
        "rm",
        "du",
        "ln",
        "stat",
        "tar",
        "gzip",
        "zip",
        "git",
        "echo",
        "printf",
        "env",
        "xargs",
        "basename",
        "dirname",
    ];
    for cmd in commands {
        eprintln!("  {cmd}");
    }
}
