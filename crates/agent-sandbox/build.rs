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

    if std::path::Path::new(&wasm_path).exists() {
        let wasm_bytes = std::fs::read(&wasm_path).expect("Failed to read toolbox.wasm");

        // AOT precompile: compile WASM to native code at build time.
        // Engine config here MUST match runtime config in runtime/mod.rs.
        let mut engine_config = wasmtime::Config::new();
        engine_config.consume_fuel(true);

        let engine =
            wasmtime::Engine::new(&engine_config).expect("Failed to create wasmtime engine");
        let precompiled = engine
            .precompile_module(&wasm_bytes)
            .expect("Failed to precompile WASM module");

        let dest = out_dir.join("toolbox.cwasm");
        std::fs::write(&dest, precompiled).expect("Failed to write precompiled module");

        println!("cargo:rustc-env=TOOLBOX_WASM_AVAILABLE=1");
        println!("cargo:rustc-env=TOOLBOX_CWASM_PATH={}", dest.display());
    } else {
        // Create a placeholder for development (allows cargo check without WASM build)
        let dest = out_dir.join("toolbox.cwasm");
        std::fs::write(&dest, b"").expect("Failed to create placeholder");

        println!("cargo:rustc-env=TOOLBOX_WASM_AVAILABLE=0");
        println!("cargo:rustc-env=TOOLBOX_CWASM_PATH={}", dest.display());
        println!(
            "cargo:warning=toolbox.wasm not found at {}. Build with: cargo build --target wasm32-wasip1 --manifest-path wasm/toolbox/Cargo.toml --release",
            wasm_path
        );
    }

    println!("cargo:rerun-if-changed={}", wasm_path);
    println!("cargo:rerun-if-env-changed=TOOLBOX_WASM_PATH");
}
