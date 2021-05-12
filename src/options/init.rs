use crate::{options::FuzzDirWrapper, project::FuzzProject, RunCommand};
use anyhow::Result;
use structopt::StructOpt;

#[derive(Clone, Debug, StructOpt)]
pub struct Init {
    #[structopt(
        short = "t",
        long = "target",
        required = false,
        default_value = "fuzz_target_1"
    )]
    /// Name of the first fuzz target to create
    pub target: String,

    #[structopt(flatten)]
    pub fuzz_dir_wrapper: FuzzDirWrapper,
}

impl RunCommand for Init {
    fn run_command(&mut self) -> Result<()> {
        FuzzProject::init(self, self.fuzz_dir_wrapper.fuzz_dir.to_owned())?;
        Ok(())
    }
}
