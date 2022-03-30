use std::env;

fn main() {
    // Cargo sets the host and target env vars for build scripts, but not crates:
    // https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts
    // So we just re-export them to the crate code.
    println!("cargo:rustc-env=TARGET={}", env::var("TARGET").unwrap());
    // By default Cargo only runs the build script when a file changes.
    // This makes it re-run on target change
    println!("cargo:rerun-if-changed-env=TARGET")
}
