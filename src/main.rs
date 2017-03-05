// Copyright 2016 rust-fuzz developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

extern crate toml;
extern crate clap;
extern crate term;
#[macro_use]
extern crate error_chain;

use clap::{App, Arg, SubCommand, ArgMatches, AppSettings};
use std::{env, fs, path, process};
use std::io::Write;
use std::io::Read;

#[macro_use]
mod templates;
mod utils;

error_chain! {
    foreign_links {
        Toml(toml::de::Error);
        Io(::std::io::Error);
    }
}

fn main() {
    let app = App::new("cargo-fuzz")
        .version(option_env!("CARGO_PKG_VERSION").unwrap_or("0.0.0"))
        .about(option_env!("CARGO_PKG_DESCRIPTION").unwrap_or(""))
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .setting(AppSettings::GlobalVersion)
        // cargo passes in the subcommand name to the invoked executable. Use a hidden, optional
        // positional argument to deal with it?
        .arg(Arg::with_name("dummy").possible_value("fuzz").required(false).hidden(true))
        .subcommand(SubCommand::with_name("init").about("Initialize the fuzz folder"))
        .subcommand(SubCommand::with_name("run").about("Run the fuzz target in fuzz/fuzzers")
            .setting(AppSettings::TrailingVarArg)
            .arg(Arg::with_name("TARGET").required(true)
                 .help("name of the fuzz target"))
            .arg(Arg::with_name("ARGS").multiple(true)
                 .help("additional libFuzzer arguments passed to the binary"))
        )
        .subcommand(SubCommand::with_name("add").about("Add a new fuzz target")
                    .arg(Arg::with_name("TARGET").required(true)))
        .subcommand(SubCommand::with_name("list").about("List all fuzz targets"));
    let args = app.get_matches();

    ::std::process::exit(match args.subcommand() {
        ("init", _) => FuzzProject::init().map(|_| ()),
        ("add", matches) =>
            FuzzProject::new().and_then(|p| p.add_target(matches.expect("arguments present"))),
        ("list", _) => FuzzProject::new().and_then(|p| p.list_targets()),
        ("run", matches) =>
            FuzzProject::new().and_then(|p| p.exec_target(matches.expect("arguments present"))),
        (s, _) => panic!("unimplemented subcommand {}!", s),
    }.map(|_| 0).unwrap_or_else(|err| {
        utils::report_error(err);
        1
    }));
}

struct FuzzProject {
    /// Path to the root cargo project
    ///
    /// Not the project with fuzz targets, but the project being fuzzed
    root_project: path::PathBuf,
    targets: Vec<String>,
}

impl FuzzProject {
    fn new() -> Result<Self> {
        let mut project = FuzzProject {
            root_project: find_package()?,
            targets: Vec::new()
        };
        let manifest = project.manifest()?;
        if !is_fuzz_manifest(&manifest) {
            return Err(format!("manifest `{:?}` does not look a cargo-fuzz manifest. \
                                Add following lines to override:\n\
                                [package.metadata]\ncargo-fuzz = true",
                                project.manifest_path()).into());
        }
        project.targets = collect_targets(&manifest);
        Ok(project)
    }

    /// Create the fuzz project structure
    ///
    /// This will not clone libfuzzer-sys
    fn init() -> Result<Self> {
        let project = FuzzProject {
            root_project: find_package()?,
            targets: Vec::new(),
        };
        let fuzz_project = project.path();
        // TODO: check if the project is already initialized
        fs::create_dir(&fuzz_project)?;
        fs::create_dir(fuzz_project.join("fuzzers"))?;

        let mut cargo = fs::File::create(fuzz_project.join("Cargo.toml"))?;
        cargo.write_fmt(toml_template!(project.root_project_name()?))?;

        let mut ignore = fs::File::create(fuzz_project.join(".gitignore"))?;
        ignore.write_fmt(gitignore_template!())?;

        const TARGET: &'static str = "fuzzer_script_1";
        project.create_target_template(TARGET)
               .chain_err(|| format!("could not create template file for target {:?}", TARGET))?;
        Ok(project)
    }

    fn list_targets(&self) -> Result<()> {
        for bin in &self.targets {
            utils::print_message(bin, term::color::GREEN);
        }
        Ok(())
    }

    fn add_target<'a>(&self, args: &ArgMatches<'a>) -> Result<()> {
        let target: String = args.value_of_os("TARGET").expect("TARGET is required").to_os_string()
            .into_string().map_err(|_| "TARGET must be valid unicode")?;
        self.create_target_template(&target)
            .chain_err(|| format!("could not create template file for target {:?}", target))
    }

    /// Add a new fuzz target script with a given name
    fn create_target_template<'a>(&self, target: &str) -> Result<()> {
        let target_path = self.target_path(target);
        let mut script = fs::OpenOptions::new().write(true).create_new(true).open(&target_path)?;
        script.write_fmt(target_template!(self.root_project_name()?.replace("-", "_")))?;

        let mut cargo = fs::OpenOptions::new().append(true)
            .open(self.manifest_path())?;
        Ok(cargo.write_fmt(toml_bin_template!(target))?)
    }

    /// Fuzz a given fuzz target
    fn exec_target<'a>(&self, args: &ArgMatches<'a>) -> Result<()> {
        let target: String = args.value_of_os("TARGET").expect("TARGET is required").to_os_string()
            .into_string().map_err(|_| "TARGET must be valid unicode")?;
        let exec_args = args.values_of_os("ARGS");
        let target_triple = "x86_64-unknown-linux-gnu";

        env::set_current_dir(self.path())?;
        let mut flags = env::var("RUSTFLAGS").unwrap_or("".into());
        if !flags.is_empty() {
            flags.push(' ');
        }
        flags.push_str("-Cpasses=sancov -Cllvm-args=-sanitizer-coverage-level=3 -Zsanitizer=address -Cpanic=abort");
        let mut cmd = process::Command::new("cargo");

        fs::create_dir_all("corpus")?;
        fs::create_dir_all("artifacts")?;

        cmd.env("RUSTFLAGS", flags)
           .arg("run")
           .arg("--verbose")
           .arg("--bin")
           .arg(&target)
           .arg("--target")
           // won't pass rustflags to build scripts
           .arg(target_triple)
           .arg("--")
           .arg("-artifact_prefix=artifacts/")
           .env("ASAN_OPTIONS", "detect_odr_violation=0");
        exec_args.map(|args| for arg in args { cmd.arg(arg); });
        cmd.arg("corpus"); // must be last arg
        exec_cmd(&mut cmd).chain_err(|| format!("could not execute command: {:?}", cmd))?;
        Ok(())
    }

    fn path(&self) -> path::PathBuf {
        self.root_project.join("fuzz")
    }

    fn manifest_path(&self) -> path::PathBuf {
        self.path().join("Cargo.toml")
    }

    fn target_path(&self, target: &str) -> path::PathBuf {
        let mut root = self.path();
        root.push("fuzzers");
        root.push(target);
        root.set_extension("rs");
        root
    }

    fn manifest(&self) -> Result<toml::Value> {
        let filename = self.manifest_path();
        let mut file = fs::File::open(&filename).chain_err(||
            format!("could not read the manifest file: {:?}", filename)
        )?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        toml::from_slice(&data).chain_err(||
            format!("could not decode the manifest file at {:?}", filename)
        )
    }

    fn root_project_name(&self) -> Result<String> {
        let filename = self.root_project.join("Cargo.toml");
        let mut file = fs::File::open(&filename)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        let value: toml::Value = toml::from_slice(&data)?;
        let name = value.as_table().and_then(|v| v.get("package"))
                                   .and_then(toml::Value::as_table)
                                   .and_then(|v| v.get("name"))
                                   .and_then(toml::Value::as_str);
        if let Some(name) = name {
            Ok(String::from(name))
        } else {
            Err(format!("{:?} (package.name) is malformed", filename).into())
        }
    }
}

fn collect_targets(value: &toml::Value) -> Vec<String> {
    let bins = value.as_table().and_then(|v| v.get("bin"))
                               .and_then(toml::Value::as_array);
    if let Some(bins) = bins {
        bins.iter().map(|bin|
            bin.as_table().and_then(|v| v.get("name")).and_then(toml::Value::as_str)
        ).filter_map(|name| name.map(|v| String::from(v))).collect()
    } else {
        Vec::new()
    }
}

fn is_fuzz_manifest(value: &toml::Value) -> bool {
    let is_fuzz = value.as_table().and_then(|v| v.get("package"))
                                  .and_then(toml::Value::as_table)
                                  .and_then(|v| v.get("metadata"))
                                  .and_then(toml::Value::as_table)
                                  .and_then(|v| v.get("cargo-fuzz"))
                                  .and_then(toml::Value::as_bool);
    is_fuzz == Some(true)
}

/// Returns the path for the first found non-fuzz Cargo package
fn find_package() -> Result<path::PathBuf> {
    let mut dir = env::current_dir()?;
    let mut data = Vec::new();
    loop {
        let manifest_path = dir.join("Cargo.toml");
        match fs::File::open(&manifest_path) {
            Err(_) => {},
            Ok(mut f) => {
                f.read_to_end(&mut data)?;
                let value: toml::Value = toml::from_slice(&data)
                    .chain_err(||
                        format!("could not decode the manifest file at {:?}", manifest_path)
                    )?;
                if !is_fuzz_manifest(&value) {
                    // Not a cargo-fuzz project => must be a proper cargo project :)
                    return Ok(dir);
                }
            }
        }
        if !dir.pop() { break; }
    }
    Err("could not find a cargo project".into())
}

#[cfg(unix)]
fn exec_cmd(cmd: &mut process::Command) -> Result<process::ExitStatus> {
    use std::os::unix::process::CommandExt;
    Err(cmd.exec().into())
}

#[cfg(not(unix))]
fn exec_cmd(cmd: &mut process::Command) -> Result<process::ExitStatus> {
    cmd.status().map_err(|e| e.into())
}
