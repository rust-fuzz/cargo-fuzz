use crate::{
    options::{BuildMode, BuildOptions, FuzzDirWrapper},
    project::FuzzProject,
    RunCommand,
};
use anyhow::Result;
use clap::Parser;

#[derive(Clone, Debug, Parser)]
pub struct Check {
    #[command(flatten)]
    pub build: BuildOptions,

    #[command(flatten)]
    pub fuzz_dir_wrapper: FuzzDirWrapper,

    /// Name of the fuzz target to check, or check all targets if not supplied
    pub target: Option<String>,
}

impl RunCommand for Check {
    fn run_command(&mut self) -> Result<()> {
        let project = FuzzProject::new(self.fuzz_dir_wrapper.fuzz_dir.to_owned())?;
        project.exec_build(BuildMode::Check, &self.build, self.target.as_deref())
    }
}
