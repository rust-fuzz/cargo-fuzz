use crate::{
    options::{BuildOptions, FuzzDirWrapper},
    project::FuzzProject,
    RunCommand,
};
use anyhow::Result;
use structopt::StructOpt;

#[derive(Clone, Debug, StructOpt)]
pub struct Run {
    #[structopt(flatten)]
    pub build: BuildOptions,

    /// Name of the fuzz target
    pub target: String,

    /// Custom corpus directories or artifact files.
    pub corpus: Vec<String>,

    #[structopt(flatten)]
    pub fuzz_dir_wrapper: FuzzDirWrapper,

    #[structopt(
        short = "j",
        long = "jobs",
        default_value = "1",
        validator(|v| Err(From::from(match v.parse::<u16>() {
            Ok(0) => "0 jobs?",
            Err(_) => "must be a valid integer representing a sane number of jobs",
            _ => return Ok(()),
        }))),
    )]
    /// Number of concurrent jobs to run
    pub jobs: u32,

    #[structopt(last(true))]
    /// Additional libFuzzer arguments passed through to the binary
    pub args: Vec<String>,
}

impl RunCommand for Run {
    fn run_command(&mut self) -> Result<()> {
        let project = FuzzProject::new(self.fuzz_dir_wrapper.fuzz_dir.to_owned())?;
        project.exec_fuzz(self)
    }
}
