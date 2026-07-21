use crate::{
    options::{BuildOptions, FuzzDirWrapper},
    project::FuzzProject,
    RunCommand,
};
use anyhow::Result;
use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Clone, Debug, Parser)]
pub struct Cmin {
    #[command(flatten)]
    pub build: BuildOptions,

    #[command(flatten)]
    pub fuzz_dir_wrapper: FuzzDirWrapper,

    /// Name of the fuzz target
    pub target: String,

    #[arg()]
    /// The corpus directory to minify into
    pub corpus: Option<PathBuf>,

    #[arg(
        short,
        long,
        default_value_t = u16::try_from(num_cpus::get().max(1)).unwrap_or(u16::MAX),
        value_parser = clap::value_parser!(u16).range(1..)
    )]
    /// Number of parallel jobs (defaults to number of CPUs)
    pub jobs: u16,

    #[arg(long, value_enum, default_value_t = CminStrategy::Speed)]
    /// Minimization strategy: `speed` finds a minset without global optimization,
    /// while `size` produces the globally optimal smallest minset.
    pub strategy: CminStrategy,

    #[arg(last(true))]
    /// Additional libFuzzer arguments passed through to the binary
    pub args: Vec<String>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum CminStrategy {
    Speed,
    Size,
}

impl RunCommand for Cmin {
    fn run_command(&mut self) -> Result<()> {
        let project = FuzzProject::new(self.fuzz_dir_wrapper.fuzz_dir.to_owned())?;
        project.exec_cmin(self)
    }
}
