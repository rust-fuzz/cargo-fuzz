/// The default target to pass to cargo, to workaround issue #11.
pub fn default_target() -> &'static str {
    env!("TARGET")
}
