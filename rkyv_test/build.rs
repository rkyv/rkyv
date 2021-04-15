use std::env;

// HACK: Tests should be run with `wasm-pack test --node -- --features "wasm"` but wasm-pack runs
// `cargo build` before `cargo test` and doesn't pass the additional arguments to the build step. To
// work around this, we just manually turn on the `wasm` feature from the build script.
// Blocking bug: https://github.com/rustwasm/wasm-pack/issues/698
fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let target = env::var("TARGET").unwrap();

    let is_wasm = target == "asmjs-unknown-emscripten"
        || target == "wasm32-unknown-emscripten"
        || target == "wasm32-unknown-unknown";

    if is_wasm {
        println!("cargo:rustc-cfg=feature=\"wasm\"");
    }
}
