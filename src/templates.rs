macro_rules! toml_template {
    ($name:expr, $edition:expr) => {
        format_args!(
            r##"[package]
name = "{name}-fuzz"
version = "0.0.0"
publish = false
{edition}
[package.metadata]
cargo-fuzz = true

[dependencies]
libfuzzer-sys = "0.4"

[dependencies.{name}]
path = ".."

# Prevent this from interfering with workspaces
[workspace]
members = ["."]

[profile.release]
debug = 1
"##,
            name = $name,
            edition = if let Some(edition) = &$edition {
                format!("edition = \"{}\"\n", edition)
            } else {
                String::new()
            },
        )
    };
}

macro_rules! toml_bin_template {
    ($name: expr) => {
        format_args!(
            r#"
[[bin]]
name = "{0}"
path = "fuzz_targets/{0}.rs"
test = false
doc = false
"#,
            $name
        )
    };
}

macro_rules! gitignore_template {
    () => {
        format_args!(
            r##"target
corpus
artifacts
coverage
"##
        )
    };
}

macro_rules! target_template {
    ($edition:expr) => {
        format_args!(
            r##"#![no_main]
{extern_crate}
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {{
    // fuzzed code goes here
}});
"##,
            extern_crate = match $edition.as_deref() {
                None | Some("2015") => "\nextern crate libfuzzer_sys;\n",
                Some(_) => "",
            },
        )
    };
}
