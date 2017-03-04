macro_rules! toml_template {
    ($name: expr) => {
format_args!(r##"
[package]
name = "{0}-fuzz"
version = "0.0.1"
authors = ["Automatically generated"]
publish = false

[package.metadata]
cargo-fuzz = true

[dependencies.{0}]
path = ".."

# Prevent this from interfering with workspaces
[workspace]
members = ["."]

[[bin]]
name = "fuzzer_script_1"
path = "fuzzers/fuzzer_script_1.rs"
"##, $name)
    }
}

macro_rules! toml_bin_template {
    ($name: expr) => {
format_args!(r#"
[[bin]]
name = "{0}"
path = "fuzzers/{0}.rs"
"#, $name)
    }
}

macro_rules! gitignore_template {
    () => {
format_args!(r##"
target
libfuzzer
corpus
artifacts
"##)
    }
}

macro_rules! target_template {
    ($name: expr) => {
format_args!(r##"#![no_main]
extern crate libfuzzer_sys;
{}
#[export_name="rust_fuzzer_test_input"]
pub extern fn go(data: &[u8]) {{
    // fuzzed code goes here
}}
"##, $name.unwrap_or(String::new()))
    }
}
