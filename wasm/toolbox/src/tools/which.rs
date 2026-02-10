pub fn run(args: &[String]) -> i32 {
    if args.is_empty() {
        return 1;
    }

    let mut exit_code = 0;
    for name in args {
        if is_known_command(name) {
            println!("/usr/bin/{}", name);
        } else {
            eprintln!("which: no {} in toolbox", name);
            exit_code = 1;
        }
    }
    exit_code
}

fn is_known_command(cmd: &str) -> bool {
    // List all commands available in the toolbox
    matches!(
        cmd,
        "cat"
            | "head"
            | "tail"
            | "touch"
            | "tee"
            | "grep"
            | "rg"
            | "find"
            | "tree"
            | "sed"
            | "sort"
            | "uniq"
            | "cut"
            | "tr"
            | "wc"
            | "jq"
            | "diff"
            | "patch"
            | "base64"
            | "sha256sum"
            | "xxd"
            | "ls"
            | "mkdir"
            | "cp"
            | "mv"
            | "rm"
            | "du"
            | "ln"
            | "stat"
            | "tar"
            | "gzip"
            | "zip"
            | "git"
            | "node"
            | "echo"
            | "printf"
            | "env"
            | "xargs"
            | "basename"
            | "dirname"
            | "sh"
            | "bash"
            | "tac"
            | "rev"
            | "nl"
            | "seq"
            | "sleep"
            | "which"
            | "whoami"
            | "hostname"
            | "printenv"
            | "readlink"
            | "rmdir"
            | "expand"
            | "unexpand"
            | "paste"
            | "comm"
            | "fold"
            | "md5sum"
            | "sha1sum"
            | "awk"
            | "column"
            | "od"
            | "split"
            | "file"
            | "date"
            | "expr"
            | "join"
            | "strings"
            | "true"
            | "false"
            | "test"
            | "["
    )
}
