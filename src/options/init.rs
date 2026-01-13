use crate::{
    options::{FuzzDirWrapper, FuzzEngine, ManifestPath},
    project::FuzzProject,
    RunCommand,
};
use anyhow::Result;
use clap::Parser;

#[derive(Clone, Debug, Parser)]
pub struct Init {
    #[arg(short, long, required = false, default_value = "fuzz_target_1")]
    /// Name of the first fuzz target to create
    pub target: String,

    #[arg(long, value_parser = clap::builder::BoolishValueParser::new(), default_value = "false")]
    /// Whether to create a separate workspace for fuzz targets crate
    pub fuzzing_workspace: Option<bool>,

    #[arg(long, default_value = "libfuzzer")]
    /// The fuzz engine that the project should use.
    ///
    /// Options: libfuzzer, libafl
    pub fuzz_engine: FuzzEngine,

    #[command(flatten)]
    pub fuzz_dir_wrapper: FuzzDirWrapper,
}

impl RunCommand for Init {
    fn run_command(&mut self) -> Result<()> {
        let manifest_path = if let Some(manifest_path) = self.fuzz_dir_wrapper.get_manifest_path() {
            manifest_path
        } else {
            let metadata = cargo_metadata::MetadataCommand::new().no_deps().exec()?;
            ManifestPath(
                metadata
                    .workspace_root
                    .to_path_buf()
                    .join("fuzz")
                    .join("Cargo.toml")
                    .into(),
            )
        };
        FuzzProject::init(manifest_path, self)?;
        Ok(())
    }
}
