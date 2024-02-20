use crate::{
    options::{BuildOptions, FuzzDirWrapper},
    project::FuzzProject,
    RunCommand,
};
use anyhow::Result;
use clap::Parser;

#[derive(Clone, Debug, Parser)]
pub struct Run {
    #[command(flatten)]
    pub build: BuildOptions,

    /// Name of the fuzz target
    pub target: String,

    /// Custom corpus directories or artifact files.
    pub corpus: Vec<String>,

    #[command(flatten)]
    pub fuzz_dir_wrapper: FuzzDirWrapper,

    #[arg(
        short,
        long,
        default_value = "1",
        value_parser = clap::value_parser!(u16).range(1..)
    )]
    /// Number of concurrent jobs to run
    pub jobs: u16,

    #[arg(last(true))]
    /// Additional libFuzzer arguments passed through to the binary
    pub args: Vec<String>,
}

impl RunCommand for Run {
    fn run_command(&mut self) -> Result<()> {
        let project = FuzzProject::new(self.fuzz_dir_wrapper.fuzz_dir.to_owned())?;
        project.exec_fuzz(self)
    }
}
