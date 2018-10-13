macro_rules! toml_template {
    ($name: expr) => {
format_args!(r##"
[package]
name = "{0}-fuzz"
version = "0.0.0"
authors = ["Automatically generated"]
publish = false

[package.metadata]
cargo-fuzz = true

[dependencies.{0}]
path = ".."
[dependencies.libfuzzer-sys]
git = "https://github.com/rust-fuzz/libfuzzer-sys.git"

# Prevent this from interfering with workspaces
[workspace]
members = ["."]
"##, $name)
    }
}

macro_rules! toml_bin_template {
    ($name: expr) => {
format_args!(r#"
[[bin]]
name = "{0}"
path = "fuzz_targets/{0}.rs"
"#, $name)
    }
}

macro_rules! gitignore_template {
    () => {
format_args!(r##"
target
corpus
artifacts
"##)
    }
}

macro_rules! target_template {
    ($name: expr) => {
format_args!(r##"#![no_main]
#[macro_use] extern crate libfuzzer_sys;
extern crate {};

fuzz_target!(|data: &[u8]| {{
    // fuzzed code goes here
}});
"##, $name)
    }
}
