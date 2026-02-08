use std::env;

pub fn run(args: &[String]) -> i32 {
    if args.is_empty() {
        // Print all environment variables
        for (key, value) in env::vars() {
            println!("{}={}", key, value);
        }
        return 0;
    }

    // Support env VAR=val command...
    let mut new_vars = Vec::new();
    let mut cmd_start = 0;

    for (i, arg) in args.iter().enumerate() {
        if let Some(eq_pos) = arg.find('=') {
            let key = &arg[..eq_pos];
            let val = &arg[eq_pos + 1..];
            new_vars.push((key.to_string(), val.to_string()));
            cmd_start = i + 1;
        } else {
            cmd_start = i;
            break;
        }
    }

    if cmd_start >= args.len() {
        // Just print specified vars
        for (key, value) in &new_vars {
            println!("{}={}", key, value);
        }
        for (key, value) in env::vars() {
            println!("{}={}", key, value);
        }
        return 0;
    }

    // We can't actually exec commands in WASM, just set and print
    for (key, value) in &new_vars {
        // SAFETY: WASM is single-threaded, so set_var is safe
        unsafe { env::set_var(key, value) };
    }
    for (key, value) in env::vars() {
        println!("{}={}", key, value);
    }

    0
}
