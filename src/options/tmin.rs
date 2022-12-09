use crate::{
    options::{BuildOptions, FuzzDirWrapper},
    project::FuzzProject,
    RunCommand,
};
use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

#[derive(Clone, Debug, Parser)]
pub struct Tmin {
    #[command(flatten)]
    pub build: BuildOptions,

    #[command(flatten)]
    pub fuzz_dir_wrapper: FuzzDirWrapper,

    /// Name of the fuzz target
    pub target: String,

    #[arg(
        short = 'r',
        long,
        default_value = "255",
        value_parser = clap::value_parser!(u32).range(1..),
    )]
    /// Number of minimization attempts to perform
    pub runs: u32,

    #[arg()]
    /// Path to the failing test case to be minimized
    pub test_case: PathBuf,

    #[arg(last(true))]
    /// Additional libFuzzer arguments passed through to the binary
    pub args: Vec<String>,
}

impl RunCommand for Tmin {
    fn run_command(&mut self) -> Result<()> {
        let project = FuzzProject::new(self.fuzz_dir_wrapper.fuzz_dir.to_owned())?;
        project.exec_tmin(self)
    }
}
