use crate::{
    options::{BuildMode, BuildOptions},
    project::FuzzProject,
    RunCommand,
};
use anyhow::Result;
use clap::Parser;

#[derive(Clone, Debug, Parser)]
pub struct Check {
    #[command(flatten)]
    pub build: BuildOptions,

    /// Name of the fuzz target to check, or check all targets if not supplied
    pub target: Option<String>,
}

impl RunCommand for Check {
    fn run_command(&mut self) -> Result<()> {
        let project = FuzzProject::new(self.build.fuzz_dir_wrapper.get_manifest_path())?;
        project.exec_build(BuildMode::Check, &self.build, self.target.as_deref())
    }
}
