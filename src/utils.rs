use std::sync::OnceLock;

/// The default target to pass to cargo, to workaround issue #11.
pub fn default_target() -> &'static str {
    // Use OnceLock because clap's default_value only accepts reference.
    static DEFAULT_TARGET: OnceLock<String> = OnceLock::new();
    DEFAULT_TARGET.get_or_init(|| {
        let config = cargo_config2::Config::load().unwrap();
        // Get the target specified in config.
        let mut targets = config.build_target_for_cli(None::<&str>).unwrap();
        if targets.len() > 1 {
            // Config can contain multiple targets, but we don't support it: https://doc.rust-lang.org/cargo/reference/config.html#buildtarget
            panic!("multi-target build is not supported: {targets:?}");
        }
        targets
            .pop()
            // Get the host triple if the target is not specified in config.
            .unwrap_or_else(|| config.host_triple().unwrap().to_owned())
    })
}
