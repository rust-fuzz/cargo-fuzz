use std::path::PathBuf;

use crate::{
    options::{BuildOptions, FuzzDirWrapper},
    project::FuzzProject,
    RunCommand,
};
use anyhow::{bail, Result};
use clap::Parser;

#[derive(Clone, Debug, Parser)]
pub struct Coverage {
    #[command(flatten)]
    pub build: BuildOptions,

    #[command(flatten)]
    pub fuzz_dir_wrapper: FuzzDirWrapper,

    /// Sets the path to the LLVM bin directory. By default, it will use the one installed with rustc
    #[arg(long)]
    pub llvm_path: Option<PathBuf>,

    /// Name of the fuzz target
    pub target: String,

    /// Custom corpus directories or artifact files
    pub corpus: Vec<String>,

    #[arg(
        short,
        long,
        default_value_t = num_cpus::get().max(1) as u16,
        value_parser = clap::value_parser!(u16).range(1..)
    )]
    /// Number of concurrent jobs to run
    pub jobs: u16,

    #[arg(last(true))]
    /// Additional libFuzzer arguments passed through to the binary
    pub args: Vec<String>,
}

impl RunCommand for Coverage {
    fn run_command(&mut self) -> Result<()> {
        if self.build.build_std {
            bail!(
                "-Zbuild-std is currently incompatible with -Zinstrument-coverage, \
                see https://github.com/rust-lang/wg-cargo-std-aware/issues/63"
            );
        }
        let project = FuzzProject::new(self.fuzz_dir_wrapper.fuzz_dir.to_owned())?;
        self.build.coverage = true;
        project.exec_coverage(self)
    }
}
