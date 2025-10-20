macro_rules! toml_template {
    ($name:expr, $edition:expr, $use_libafl:expr, $fuzzing_workspace:expr) => {
        format_args!(
            r##"[package]
name = "{name}-fuzz"
version = "0.0.0"
publish = false
{edition}
[package.metadata]
cargo-fuzz = true

[dependencies]
{libfuzzer_sys_dep}

[dependencies.{name}]
path = ".."
{workspace}"##,
            name = $name,
            edition = if let Some(edition) = &$edition {
                format!("edition = \"{}\"\n", edition)
            } else {
                String::new()
            },
            libfuzzer_sys_dep = if let Some(true) = $use_libafl {
                r##"libfuzzer-sys = { version = "0.15.3", package = "libafl_libfuzzer" }"##
            } else {
                r##"libfuzzer-sys = "0.4""##
            },
            workspace = if let Some(true) = $fuzzing_workspace {
                r##"
# Use independent workspace for fuzzers
[workspace]
members = ["."]
"##
            } else {
                ""
            }
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
bench = false
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
