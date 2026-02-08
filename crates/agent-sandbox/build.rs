use std::env;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Look for pre-built toolbox.wasm in the wasm/toolbox/target directory
    // or fall back to a TOOLBOX_WASM_PATH environment variable
    let wasm_path = env::var("TOOLBOX_WASM_PATH").ok().unwrap_or_else(|| {
        let workspace_root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        workspace_root
            .join("wasm/toolbox/target/wasm32-wasip1/release/toolbox.wasm")
            .to_string_lossy()
            .to_string()
    });

    let dest = out_dir.join("toolbox.wasm");

    // Copy the WASM binary to OUT_DIR if it exists
    if std::path::Path::new(&wasm_path).exists() {
        std::fs::copy(&wasm_path, &dest).expect("Failed to copy toolbox.wasm");
        println!("cargo:rustc-env=TOOLBOX_WASM_AVAILABLE=1");
    } else {
        // Create a placeholder for development (allows cargo check without WASM build)
        // The actual WASM binary must be built before running tests
        std::fs::write(&dest, b"").expect("Failed to create placeholder toolbox.wasm");
        println!("cargo:rustc-env=TOOLBOX_WASM_AVAILABLE=0");
        println!(
            "cargo:warning=toolbox.wasm not found at {}. Build with: cargo build --target wasm32-wasip1 -p agent-toolbox --release",
            wasm_path
        );
    }

    println!("cargo:rustc-env=TOOLBOX_WASM_PATH={}", dest.display());
    println!("cargo:rerun-if-changed={}", wasm_path);
    println!("cargo:rerun-if-env-changed=TOOLBOX_WASM_PATH");
}
