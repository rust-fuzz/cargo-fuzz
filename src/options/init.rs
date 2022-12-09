use crate::{options::FuzzDirWrapper, project::FuzzProject, RunCommand};
use anyhow::Result;
use clap::Parser;

#[derive(Clone, Debug, Parser)]
pub struct Init {
    #[arg(short, long, required = false, default_value = "fuzz_target_1")]
    /// Name of the first fuzz target to create
    pub target: String,

    #[command(flatten)]
    pub fuzz_dir_wrapper: FuzzDirWrapper,
}

impl RunCommand for Init {
    fn run_command(&mut self) -> Result<()> {
        FuzzProject::init(self, self.fuzz_dir_wrapper.fuzz_dir.to_owned())?;
        Ok(())
    }
}
