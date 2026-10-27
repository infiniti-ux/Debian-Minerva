use std::process::Command;

fn main() {
    let version = Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().replace('\n', " "))
        .unwrap_or_else(|| "rustc (unknown)".to_string());
    println!("cargo:rustc-env=MINERVA_RUSTC_VERSION={}", version);
    println!("cargo:rerun-if-changed=build.rs");
}
