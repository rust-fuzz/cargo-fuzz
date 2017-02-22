// Copyright 2016 rust-fuzz developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

extern crate cargo_metadata;
extern crate docopt;
extern crate rustc_serialize;

use cargo_metadata::metadata;
use docopt::Docopt;
use std::{env, fs, io, path, process};
use std::io::Write;

const USAGE: &'static str = "
Cargo Fuzz

Usage:
  cargo fuzz --init
  cargo fuzz --fuzz-target TARGET
  cargo fuzz --add TARGET
  cargo fuzz (-h | --help)

Options:
  -h --help              Show this screen.
  --init                 Initialize fuzz folder
  --fuzz-target TARGET   Run with given fuzz target in fuzz/fuzzers
  --add TARGET           Add a new fuzz target
";

#[derive(Debug, RustcDecodable)]
struct Args {
    flag_init: bool,
    flag_add: Option<String>,
    flag_fuzz_target: Option<String>,
}

fn main() {
    let args: Args = Docopt::new(USAGE)
                            .and_then(|d| d.decode())
                            .unwrap_or_else(|e| e.exit());

    let result = if args.flag_init {
        init_fuzz()
    } else if let Some(target) = args.flag_add {
        add_target(target)
    } else if let Some(target) = args.flag_fuzz_target {
        let result = run_target(target);
        if let Ok(success) = result {
            if success {
                // Can this ever happen?
                Ok(())
            } else {
                println!("Fuzzing found errors!");
                process::exit(-1)
            }
        } else {
            result.map(|_| ())
        }
    } else {
        println!("Invalid arguments. Usage:\n{}", USAGE);
        return;
    };
    if let Err(error) = result {
        println!("Error: {:?}", error);
    }
}

/// Create all the files and folders we need to run
///
/// This will not clone libfuzzer-sys
fn init_fuzz() -> io::Result<()> {
    // todo error handling
    let meta = metadata(None).unwrap();
    let mut p = env::current_dir().unwrap();
    p.push("Cargo.toml");
    let p = p.to_str().unwrap();
    let me = meta.packages.iter().find(|package| package.manifest_path == p).unwrap();

    fs::create_dir("./fuzz")?;
    fs::create_dir("./fuzz/fuzzers")?;

    let mut cargo = fs::File::create(path::Path::new("./fuzz/Cargo.toml"))?;

write!(cargo, r#"
[package]
name = "{0}-fuzz"
version = "0.0.1"
authors = ["Automatically generated"]

[dependencies.{0}]
path = ".."

# Prevent this from interfering with workspaces
[workspace]
members = ["."]

[[bin]]
name = "fuzzer_script_1"
path = "fuzzers/fuzzer_script_1.rs"
"#, me.name)?;

    let mut ignore = fs::File::create(path::Path::new("./fuzz/.gitignore"))?;

write!(ignore, r#"
target
libfuzzer
"#)?;

    let mut script = fs::File::create(path::Path::new("./fuzz/fuzzers/fuzzer_script_1.rs"))?;
    dummy_target(&mut script)
}

/// Create a dummy fuzz target script at the given path
fn dummy_target(script: &mut fs::File) -> io::Result<()> {
write!(script, "{}", r#"#![no_main]


extern crate fuzzer_sys;

#[export_name="rust_fuzzer_test_input"]
pub extern fn go(data: &[u8]) {
    // fuzzed code goes here
}"#)
}

/// Add a new fuzz target script with a given name
fn add_target(target: String) -> io::Result<()> {
    let target_file = format!("fuzz/fuzzers/{}.rs", target);
    let mut script = fs::File::create(path::Path::new(&target_file))?;
    dummy_target(&mut script)?;

    let mut cargo = fs::OpenOptions::new().append(true).open(path::Path::new("./fuzz/Cargo.toml"))?;

write!(cargo, r#"
[[bin]]
name = "{0}"
path = "fuzzers/{0}.rs"
"#, target)

}

/// Build or rebuild libFuzzer (rebuilds only if the compiler version changed)
///
/// We can't just use libFuzzer as a dependency since libgcc will
/// get compiled with sanitizer support. RUSTFLAGS does not discriminate
/// between build dependencies and regular ones.
///
/// https://github.com/rust-lang/cargo/issues/3739
fn rebuild_libfuzzer() -> io::Result<()> {
    if let Err(_) = env::set_current_dir("./libfuzzer") {
        let mut git = process::Command::new("git");
        let mut cmd = git.arg("clone")
                         .arg("https://github.com/rust-fuzz/libfuzzer-sys.git")
                         .arg("libfuzzer");
        let result = cmd.spawn()?.wait()?;
        if !result.success() {
            return Err(io::Error::new(io::ErrorKind::Other,
                                      "Failed to clone libfuzzer-sys"))
        }
        env::set_current_dir("./libfuzzer")?;
    }
    let mut cmd = process::Command::new("cargo");
    cmd.arg("build")
       .arg("--release")
       .spawn()?
       .wait()?;

    let result = cmd.spawn()?.wait()?;
    if !result.success() {
        return Err(io::Error::new(io::ErrorKind::Other,
                                  "Failed to build libfuzzer-sys"))
    }
    env::set_current_dir("..")
}

/// Fuzz a given fuzz target
fn run_target(target: String) -> io::Result<bool> {
    env::set_current_dir("./fuzz")?;
    rebuild_libfuzzer()?;
    let mut flags = env::var("RUSTFLAGS").unwrap_or("".into());
    if !flags.is_empty() {
        flags.push(' ');
    }
    flags.push_str("-Cpasses=sancov -Cllvm-args=-sanitizer-coverage-level=3 -Zsanitizer=address -Cpanic=abort");
    let mut cmd = process::Command::new("cargo");
    cmd.arg("rustc")
       .arg("--verbose")
       .arg("--bin")
       .arg(&target)
       .arg("--")
       .arg("-L")
       .arg("libfuzzer/target/release")
       .env("RUSTFLAGS", &flags);

    let result = cmd.spawn()?.wait()?;
    if !result.success() {
        return Err(io::Error::new(io::ErrorKind::Other, "Failed to build fuzz target"))
    }

    // can't use cargo run since we can't pass -L args to it
    let path = format!("target/debug/{}", target);
    let mut run_cmd = process::Command::new(path);
    let result = run_cmd.spawn()?.wait()?;
    Ok(result.success())
}
