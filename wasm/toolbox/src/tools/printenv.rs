use std::env;

pub fn run(args: &[String]) -> i32 {
    if args.is_empty() {
        for (key, value) in env::vars() {
            println!("{}={}", key, value);
        }
        return 0;
    }

    let mut exit_code = 0;
    for name in args {
        match env::var(name) {
            Ok(value) => println!("{}", value),
            Err(_) => exit_code = 1,
        }
    }
    exit_code
}
