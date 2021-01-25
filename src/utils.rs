/// The default target to pass to cargo, to workaround issue #11.
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub fn default_target() -> &'static str {
    "aarch64-apple-darwin"
}

#[cfg(all(not(target_arch = "aarch64"), target_os = "macos"))]
pub fn default_target() -> &'static str {
    "x86_64-apple-darwin"
}

/// The default target to pass to cargo, to workaround issue #11.
#[cfg(not(target_os = "macos"))]
pub fn default_target() -> &'static str {
    "x86_64-unknown-linux-gnu"
}
