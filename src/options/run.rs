use crate::{options::BuildOptions, project::FuzzProject, RunCommand};
use anyhow::Result;
use structopt::StructOpt;

#[derive(Clone, Debug, StructOpt)]
pub struct Run {
    #[structopt(flatten)]
    pub build: BuildOptions,

    /// Custom corpus directories or artifact files.
    pub corpus: Vec<String>,

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
        let project = FuzzProject::find_existing()?;
        project.exec_fuzz(self)
    }
}
