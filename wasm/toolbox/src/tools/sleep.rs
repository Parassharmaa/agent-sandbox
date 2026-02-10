pub fn run(args: &[String]) -> i32 {
    if args.is_empty() {
        eprintln!("sleep: missing operand");
        return 1;
    }

    // In WASM sandbox, sleep is a no-op but we accept the argument for compatibility
    for arg in args {
        let s = arg.trim_end_matches('s');
        if s.parse::<f64>().is_err() {
            eprintln!("sleep: invalid time interval '{}'", arg);
            return 1;
        }
    }

    // No actual sleep in WASM — just return success
    0
}
