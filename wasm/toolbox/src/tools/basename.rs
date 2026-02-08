use std::path::Path;

pub fn run(args: &[String]) -> i32 {
    if args.is_empty() {
        eprintln!("basename: missing operand");
        return 1;
    }

    let path = &args[0];
    let suffix = args.get(1).map(|s| s.as_str());

    let name = Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.clone());

    let result = if let Some(suffix) = suffix {
        if name.ends_with(suffix) && name.len() > suffix.len() {
            name[..name.len() - suffix.len()].to_string()
        } else {
            name
        }
    } else {
        name
    };

    println!("{}", result);
    0
}
