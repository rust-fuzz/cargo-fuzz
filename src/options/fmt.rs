use crate::{options::BuildOptions, project::FuzzProject, RunCommand};
use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

#[derive(Clone, Debug, Parser)]
pub struct Fmt {
    #[command(flatten)]
    pub build: BuildOptions,

    /// Name of fuzz target
    pub target: String,

    /// Path to the input testcase to debug print
    pub input: PathBuf,
}

impl RunCommand for Fmt {
    fn run_command(&mut self) -> Result<()> {
        let project = FuzzProject::new(self.build.fuzz_dir_wrapper.get_manifest_path())?;
        project.debug_fmt_input(self)
    }
}
