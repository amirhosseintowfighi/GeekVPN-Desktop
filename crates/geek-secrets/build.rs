fn main() {
    println!("cargo:rerun-if-env-changed=GEEK_DEVICE_ID_KEY");
    if let Ok(v) = std::env::var("GEEK_DEVICE_ID_KEY") {
        if !v.is_empty() {
            println!("cargo:rustc-env=GEEK_DEVICE_ID_KEY={v}");
        }
    }
}
