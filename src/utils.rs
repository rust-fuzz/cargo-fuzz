use std::ffi::OsString;

/// The default target to pass to cargo, to workaround issue #11.
pub fn default_target() -> &'static str {
    current_platform::CURRENT_PLATFORM
}

/// Gets the path to the asan DLL required for the asan instrumented binary to run.
#[cfg(target_env = "msvc")]
pub fn get_asan_path() -> Option<std::path::PathBuf> {
    // The asan DLL sits next to cl & link.exe. So grab the parent path.
    Some(
        cc::windows_registry::find_tool(default_target(), "link.exe")?
            .path()
            .parent()?
            .to_owned(),
    )
}

/// Append a value to the PATH variable
#[cfg(target_env = "msvc")]
pub fn append_to_pathvar(path: &std::path::Path) -> Option<OsString> {
    use std::env;

    if let Some(current) = env::var_os("PATH") {
        let mut current = env::split_paths(&current).collect::<Vec<_>>();
        current.push(path.to_path_buf());
        return env::join_paths(current).ok();
    }

    return None;
}
