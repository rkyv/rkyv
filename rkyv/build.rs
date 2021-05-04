use std::env;

fn main() {
    let target = env::var("TARGET").unwrap();

    let is_wasm = target == "asmjs-unknown-emscripten"
        || target == "wasm32-unknown-emscripten"
        || target == "wasm32-unknown-unknown";

    let has_atomic64 = target.starts_with("x86_64")
        || target.starts_with("i686")
        || target.starts_with("aarch64")
        || target.starts_with("powerpc64")
        || target.starts_with("sparc64")
        || target.starts_with("mips64el");
    let has_atomic32 = has_atomic64 || is_wasm;

    if has_atomic64 {
        println!("cargo:rustc-cfg=has_atomics_64");
    }

    if has_atomic32 {
        println!("cargo:rustc-cfg=has_atomics");
    }
}
