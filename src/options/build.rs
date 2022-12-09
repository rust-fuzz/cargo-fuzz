use crate::{
    options::{BuildMode, BuildOptions, FuzzDirWrapper},
    project::FuzzProject,
    RunCommand,
};
use anyhow::Result;
use clap::Parser;

#[derive(Clone, Debug, Parser)]
pub struct Build {
    #[command(flatten)]
    pub build: BuildOptions,

    #[command(flatten)]
    pub fuzz_dir_wrapper: FuzzDirWrapper,

    /// Name of the fuzz target to build, or build all targets if not supplied
    pub target: Option<String>,
}

impl RunCommand for Build {
    fn run_command(&mut self) -> Result<()> {
        let project = FuzzProject::new(self.fuzz_dir_wrapper.fuzz_dir.to_owned())?;
        project.exec_build(BuildMode::Build, &self.build, self.target.as_deref())
    }
}
