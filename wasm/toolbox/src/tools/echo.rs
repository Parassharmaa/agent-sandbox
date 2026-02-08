pub fn run(args: &[String]) -> i32 {
    let mut no_newline = false;
    let mut start = 0;

    if !args.is_empty() && args[0] == "-n" {
        no_newline = true;
        start = 1;
    }

    let output = args[start..].join(" ");
    if no_newline {
        print!("{}", output);
    } else {
        println!("{}", output);
    }

    0
}
