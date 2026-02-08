use std::path::Path;

pub fn run(args: &[String]) -> i32 {
    if args.is_empty() {
        eprintln!("dirname: missing operand");
        return 1;
    }

    for arg in args {
        let dir = Path::new(arg)
            .parent()
            .map(|p| {
                let s = p.to_string_lossy().to_string();
                if s.is_empty() { ".".to_string() } else { s }
            })
            .unwrap_or_else(|| ".".to_string());
        println!("{}", dir);
    }

    0
}
