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
  cargo fuzz --script SCRIPT
  cargo fuzz --add SCRIPT
  cargo fuzz (-h | --help)

Options:
  -h --help         Show this screen.
  --init            Initialize fuzz folder
  --script SCRIPT   Run with given fuzz script in fuzz/fuzzers
  --add SCRIPT      Add a new fuzz script
";

#[derive(Debug, RustcDecodable)]
struct Args {
    flag_init: bool,
    flag_add: Option<String>,
    flag_script: Option<String>,
}

fn main() {
    let args: Args = Docopt::new(USAGE)
                            .and_then(|d| d.decode())
                            .unwrap_or_else(|e| e.exit());

    let result = if args.flag_init {
        init_fuzz()
    } else if let Some(script) = args.flag_add {
        add_script(script)
    } else if let Some(script) = args.flag_script {
        let result = run_script(script);
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
    dummy_script(&mut script)
}

/// Create a dummy fuzz script script at the given path
fn dummy_script(script: &mut fs::File) -> io::Result<()> {
write!(script, r#"
#![no_main]
extern crate fuzzer_sys;

#[export_name="LLVMFuzzerTestOneInput"]
pub extern fn go(data: *const u8, size: isize) -> i32 {{
    // fuzzed code goes here
    0
}}"#)
}

/// Add a new fuzz script script with a given name
fn add_script(script_name: String) -> io::Result<()> {
    let script = format!("fuzz/fuzzers/{}.rs", script_name);
    let mut script = fs::File::create(path::Path::new(&script))?;
    dummy_script(&mut script)?;

    let mut cargo = fs::OpenOptions::new().append(true).open(path::Path::new("./fuzz/Cargo.toml"))?;

    write!(cargo, r#"\
    [[bin]]\
    name = "{0}"\
    path = "fuzzers/{0}.rs"\
    "#, script_name)
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

/// Fuzz a given fuzz script
fn run_script(script: String) -> io::Result<bool> {
    env::set_current_dir("./fuzz")?;
    rebuild_libfuzzer()?;
    let mut cmd = process::Command::new("cargo");
    cmd.arg("rustc")
       .arg("--verbose")
       .arg("--bin")
       .arg(&script)
       .arg("--")
       .arg("-L")
       .arg("libfuzzer/target/release")
       .env("RUSTFLAGS",
            "-Cpasses=sancov -Cllvm-args=-sanitizer-coverage-level=3 -Zsanitizer=address -Cpanic=abort");

    let result = cmd.spawn()?.wait()?;
    if !result.success() {
        return Err(io::Error::new(io::ErrorKind::Other, "Failed to build fuzz script"))
    }

    // can't use cargo run since we can't pass -L args to it
    let path = format!("target/debug/{}", script);
    let mut run_cmd = process::Command::new(path);
    let result = run_cmd.spawn()?.wait()?;
    Ok(result.success())
}
