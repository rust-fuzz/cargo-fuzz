// Copyright 2016 rust-fuzz developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use anyhow::Result;
use clap::Parser;

#[macro_use]
mod templates;
mod options;
mod project;
mod rustc_version;
mod utils;

static FUZZ_TARGETS_DIR_OLD: &str = "fuzzers";
static FUZZ_TARGETS_DIR: &str = "fuzz_targets";

// It turns out that `clap`'s `long_about()` makes `cargo fuzz --help`
// unreadable, and its `before_help()` injects our long about text before the
// version, so change the default template slightly.
const LONG_ABOUT_TEMPLATE: &str = "\
{bin} {version}
{about}

USAGE:
    {usage}

{before-help}

{all-args}

{after-help}";

const RUN_BEFORE_HELP: &str = "\
The fuzz target name is the same as the name of the fuzz target script in
fuzz/fuzz_targets/, i.e. the name picked when running `cargo fuzz add`.

This will run the script inside the fuzz target with varying inputs until it
finds a crash, at which point it will save the crash input to the artifact
directory, print some output, and exit. Unless you configure it otherwise (see
libFuzzer options below), this will run indefinitely.

By default fuzz targets are built with optimizations equivalent to
`cargo build --release`, but with debug assertions and overflow checks enabled.
Address Sanitizer is also enabled by default.";

const RUN_AFTER_HELP: &str = "\
A full list of libFuzzer options can be found at
http://llvm.org/docs/LibFuzzer.html#options

You can also get this by running `cargo fuzz run fuzz_target -- -help=1`

Some useful options (to be used as `cargo fuzz run fuzz_target -- <options>`)
include:

  * `-max_len=<len>`: Will limit the length of the input string to `<len>`

  * `-runs=<number>`: Will limit the number of tries (runs) before it gives up

  * `-max_total_time=<time>`: Will limit the amount of time (seconds) to
    fuzz before it gives up

  * `-timeout=<time>`: Will limit the amount of time (seconds) for a single
    run before it considers that run a failure

  * `-only_ascii`: Only provide ASCII input

  * `-dict=<file>`: Use a keyword dictionary from specified file. See
    http://llvm.org/docs/LibFuzzer.html#dictionaries\
";

const BUILD_BEFORE_HELP: &str = "\
By default fuzz targets are built with optimizations equivalent to
`cargo build --release`, but with debug assertions and overflow checks enabled.
Address Sanitizer is also enabled by default.";

const BUILD_AFTER_HELP: &str = "\
Sanitizers perform checks necessary for detecting bugs in unsafe code
at the cost of some performance. For more information on sanitizers see
https://doc.rust-lang.org/unstable-book/compiler-flags/sanitizer.html\
";

/// A trait for running our various commands.
trait RunCommand {
    /// Run this command!
    fn run_command(&mut self) -> Result<()>;
}

#[derive(Clone, Debug, Parser)]
#[command(version, about)]
#[command(subcommand_required = true)]
#[command(arg_required_else_help = true)]
#[command(propagate_version = true)]
// Cargo passes in the subcommand name to the invoked executable.
// Use a hidden, optional positional argument to deal with it.
#[command(
    arg(clap::Arg::new("dummy")
        .value_parser(["fuzz"])
        .required(false)
        .hide(true))
)]
enum Command {
    /// Initialize the fuzz directory
    Init(options::Init),

    /// Add a new fuzz target
    Add(options::Add),

    #[command(
        help_template(LONG_ABOUT_TEMPLATE),
        before_help(BUILD_BEFORE_HELP),
        after_help(BUILD_AFTER_HELP),
        visible_alias("b")
    )]
    /// Build fuzz targets
    Build(options::Build),

    #[command(help_template(LONG_ABOUT_TEMPLATE), visible_alias("c"))]
    /// Type-check the fuzz targets
    Check(options::Check),

    /// Print the `std::fmt::Debug` output for an input
    Fmt(options::Fmt),

    #[command(visible_alias("ls"))]
    /// List all the existing fuzz targets
    List(options::List),

    #[command(
        help_template(LONG_ABOUT_TEMPLATE),
        before_help(RUN_BEFORE_HELP),
        after_help(RUN_AFTER_HELP),
        visible_alias("r")
    )]
    /// Run a fuzz target
    Run(options::Run),

    /// Minify a corpus
    Cmin(options::Cmin),

    /// Minify a test case
    Tmin(options::Tmin),

    #[command(visible_alias("cov"))]
    /// Run program on the generated corpus and generate coverage information
    Coverage(options::Coverage),
}

impl RunCommand for Command {
    fn run_command(&mut self) -> Result<()> {
        match self {
            Command::Init(x) => x.run_command(),
            Command::Add(x) => x.run_command(),
            Command::Build(x) => x.run_command(),
            Command::Check(x) => x.run_command(),
            Command::List(x) => x.run_command(),
            Command::Fmt(x) => x.run_command(),
            Command::Run(x) => x.run_command(),
            Command::Cmin(x) => x.run_command(),
            Command::Tmin(x) => x.run_command(),
            Command::Coverage(x) => x.run_command(),
        }
    }
}

fn main() -> Result<()> {
    Command::parse().run_command()
}
