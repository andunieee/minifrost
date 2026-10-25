// `c++_static` (see .cargo/config.toml) only pulls in libc++; the exception personality
// (`__gxx_personality_v0`) lives in libc++abi, which must be linked explicitly or the .so
// loads only on devices where some other library happens to provide the symbol.
fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android") {
        println!("cargo:rustc-link-lib=c++abi");
    }
}
