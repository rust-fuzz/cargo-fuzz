// Copyright 2016 rust-fuzz developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

extern crate cargo_metadata;
extern crate docopt;
extern crate rustc_serialize;
extern crate term;

use cargo_metadata::{metadata, Package};
use docopt::Docopt;
use std::{env, error, fs, io, path, process};
use std::io::Write;

const USAGE: &'static str = "
Cargo Fuzz

Usage:
  cargo fuzz --init
  cargo fuzz --fuzz-target TARGET
  cargo fuzz --add TARGET
  cargo fuzz --list
  cargo fuzz (-h | --help)

Options:
  -h --help              Show this screen.
  --init                 Initialize fuzz folder
  --fuzz-target TARGET   Run with given fuzz target in fuzz/fuzzers
  --add TARGET           Add a new fuzz target
  --list                 List the available fuzz targets
";

#[derive(Debug, RustcDecodable)]
struct Args {
    flag_init: bool,
    flag_add: Option<String>,
    flag_fuzz_target: Option<String>,
    flag_list: bool,
}

fn main() {
    let mut term_stdout = term::stdout();
    let mut term_stderr = term::stderr();
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
                if let Some(ref mut terminal) = term_stdout {
                    let _ = terminal.fg(term::color::YELLOW);
                    println!("Fuzzing found errors!");
                    let _ = terminal.reset();
                } else {
                    println!("Fuzzing found errors!");
                }
                process::exit(-1)
            }
        } else {
            result.map(|_| ())
        }
    } else if args.flag_list {
        list_fuzz_targets(&mut term_stdout)
            .map(|_| ())
    } else {
        if let Some(ref mut terminal) = term_stderr {
            let _ = terminal.attr(term::Attr::Bold);
            let _ = terminal.fg(term::color::RED);
            write!(io::stderr(), "Error:")
                .expect("failed writing to stderr");
            let _ = terminal.fg(term::color::WHITE);
            writeln!(io::stderr(), " Invalid arguments. Usage:\n{}", USAGE)
                .expect("failed writing to stderr");
            let _ = terminal.reset();
        } else {
            writeln!(io::stderr(), "Invalid arguments. Usage:\n{}", USAGE)
                .expect("failed writing to stderr");
        }

        return;
    };
    if let Err(error) = result {
        if let Some(ref mut terminal) = term_stderr {
            let _ = terminal.attr(term::Attr::Bold);
            let _ = terminal.fg(term::color::RED);
            write!(io::stderr(), "Error: ")
                .expect("failed writing to stderr");
            let _ = terminal.fg(term::color::WHITE);
            writeln!(io::stderr(), "{}", error)
                .expect("failed writing to stderr");
            let _ = terminal.reset();
        } else {
            writeln!(io::stderr(), "Error: {}", error)
                .expect("failed writing to stderr");
        }
    }
}

fn list_fuzz_targets(terminal: &mut Option<Box<term::StdoutTerminal>>) -> Result<(), Box<error::Error>> {
    if !path::Path::new("./fuzz").is_dir() {
        return Err("Fuzzing crate has not been initialized. Run `cargo fuzz --init` to initialize it.".into());
    }

    if let Some(ref mut term_stdout) = *terminal {
        let _ = term_stdout.fg(term::color::GREEN);
    }
    env::set_current_dir("./fuzz")?;
    let package = get_package();
    for target in &package.targets {
        println!("{}", target.name);
    }

    if let Some(ref mut term_stdout) = *terminal {
        let _ = term_stdout.reset();
    }

    Ok(())
}

/// Create all the files and folders we need to run
///
/// This will not clone libfuzzer-sys
fn init_fuzz() -> Result<(), Box<error::Error>> {
    let me = get_package();

    fs::create_dir("./fuzz")?;
    fs::create_dir("./fuzz/fuzzers")?;

    let mut cargo = fs::File::create(path::Path::new("./fuzz/Cargo.toml"))?;

write!(cargo, r#"
[package]
name = "{0}-fuzz"
version = "0.0.1"
authors = ["Automatically generated"]
publish = false

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
corpus
artifacts
"#)?;

    let mut script = fs::File::create(path::Path::new("./fuzz/fuzzers/fuzzer_script_1.rs"))?;
    dummy_target(&mut script, &me)
}

/// Returns metadata for the Cargo package in the current directory
fn get_package() -> Package {
    // todo error handling
    let meta = metadata(None).unwrap();
    let mut p = env::current_dir().unwrap();
    p.push("Cargo.toml");
    let p = p.to_str().unwrap();
    meta.packages.into_iter().find(|package| package.manifest_path == p).unwrap()
}

/// If the package contains a library target, generate an `extern crate` line to link to it.
fn link_to_lib(pkg: &Package) -> Option<String> {
    pkg.targets.iter()
               .find(|target| target.kind.iter().any(|k| k == "lib"))
               .map(|target| format!("extern crate {};\n", target.name.replace("-", "_")))
}

/// Create a dummy fuzz target script at the given path
fn dummy_target(script: &mut fs::File, pkg: &Package) -> Result<(), Box<error::Error>> {
write!(script, r#"#![no_main]
extern crate libfuzzer_sys;
{}
#[export_name="rust_fuzzer_test_input"]
pub extern fn go(data: &[u8]) {{
    // fuzzed code goes here
}}"#, link_to_lib(pkg).unwrap_or(String::new())).map_err(|e| e.into())
}

/// Add a new fuzz target script with a given name
fn add_target(target: String) -> Result<(), Box<error::Error>> {
    let target_file = format!("fuzz/fuzzers/{}.rs", target);
    let mut script = fs::File::create(path::Path::new(&target_file))?;
    let me = get_package();
    dummy_target(&mut script, &me)?;

    let mut cargo = fs::OpenOptions::new().append(true).open(path::Path::new("./fuzz/Cargo.toml"))?;

write!(cargo, r#"
[[bin]]
name = "{0}"
path = "fuzzers/{0}.rs"
"#, target).map_err(|e| e.into())

}

/// Build or rebuild libFuzzer (rebuilds only if the compiler version changed)
///
/// We can't just use libFuzzer as a dependency since libgcc will
/// get compiled with sanitizer support. RUSTFLAGS does not discriminate
/// between build dependencies and regular ones.
///
/// https://github.com/rust-lang/cargo/issues/3739
fn rebuild_libfuzzer() -> Result<(), Box<error::Error>> {
    if let Err(_) = env::set_current_dir("./libfuzzer") {
        let mut git = process::Command::new("git");
        let mut cmd = git.arg("clone")
                         .arg("https://github.com/rust-fuzz/libfuzzer-sys.git")
                         .arg("libfuzzer");
        let result = cmd.spawn()?.wait()?;
        if !result.success() {
            return Err("Failed to clone libfuzzer-sys".into())
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
        return Err("Failed to build libfuzzer-sys".into())
    }
    env::set_current_dir("..")
        .map_err(|e| e.into())
}

fn make_dir_if_not_exist(dir: &str) -> Result<(), io::Error> {
    if let Err(k) = fs::create_dir(dir) {
        if k.kind() == io::ErrorKind::AlreadyExists {
            // do nothing
        } else {
            return Err(k);
        }
    }
    Ok(())
}
/// Fuzz a given fuzz target
fn run_target(target: String) -> Result<bool, Box<error::Error>> {
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
       .arg("--target")
       .arg("x86_64-unknown-linux-gnu") // won't pass rustflags to build scripts
       .arg("--")
       .arg("-L")
       .arg("libfuzzer/target/release")
       .env("RUSTFLAGS", &flags);

    let result = cmd.spawn()?.wait()?;
    if !result.success() {
        return Err("Failed to build fuzz target".into())
    }

    make_dir_if_not_exist("corpus")?;
    make_dir_if_not_exist("artifacts")?;

    // can't use cargo run since we can't pass -L args to it
    let path = format!("target/debug/{}", target);
    let mut run_cmd = process::Command::new(path);
    run_cmd.arg("-artifact_prefix=artifacts/")
           .arg("corpus") // must be last arg
           .env("ASAN_OPTIONS", "detect_odr_violation=0");
    let result = run_cmd.spawn()?.wait()?;
    Ok(result.success())
}
