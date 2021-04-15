use std::env;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let target = env::var("TARGET").unwrap();

    let emscripten = target == "asmjs-unknown-emscripten"
        || target == "wasm32-unknown-emscripten"
        || target == "wasm32-unknown-unknown";

    if !emscripten {
        println!("cargo:rustc-cfg=feature=\"not_wasm\"");
    }

    if target == "wasm32-unknown-unknown" {
        println!("cargo:rustc-cfg=wasm_bindgen");
    }

    let has_atomic64 = target.starts_with("x86_64")
        || target.starts_with("i686")
        || target.starts_with("aarch64")
        || target.starts_with("powerpc64")
        || target.starts_with("sparc64")
        || target.starts_with("mips64el");
    let has_atomic32 = has_atomic64 || emscripten;

    if has_atomic64 {
        println!("cargo:rustc-cfg=rkyv_atomic_64");
    }

    if has_atomic32 {
        println!("cargo:rustc-cfg=rkyv_atomic");
    }
}
