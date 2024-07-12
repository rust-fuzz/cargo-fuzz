//! Rust compiler version detection

use std::{cmp::Ordering, process::Command, str::FromStr};

use anyhow::Context;

/// Checks if the compiler currently in use is nightly, or `RUSTC_BOOTSTRAP` is set to get nightly features on stable
pub fn is_nightly(version_string: &str) -> bool {
    version_string.contains("-nightly ") || std::env::var_os("RUSTC_BOOTSTRAP").is_some()
}

/// Returns the output of `rustc --version`
pub fn rust_version_string() -> anyhow::Result<String> {
    // The path to rustc can be specified via an environment variable:
    // https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-reads
    let rustc_path = std::env::var_os("RUSTC").unwrap_or("rustc".into());
    let raw_output = Command::new(rustc_path)
        .arg("--version")
        .output()
        .context("Failed to invoke rustc! Is it in your $PATH?")?
        .stdout;
    String::from_utf8(raw_output).context("`rustc --version` returned non-text output somehow")
}

/// Returns either "-Zsanitizer" or "-Csanitizer" depending on the compiler version.
///
/// Stabilization of sanitizers has removed the "-Zsanitizer" flag, even on nightly,
/// so we have to choose the appropriate flag for the compiler version.
/// More info: <https://github.com/rust-lang/rust/pull/123617>
pub fn sanitizer_flag(version: &RustVersion) -> anyhow::Result<&'static str> {
    match version.has_sanitizers_on_stable() {
        true => Ok("-Csanitizer"),
        false => Ok("-Zsanitizer"),
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd)]
pub struct RustVersion {
    pub major: u32,
    pub minor: u32,
    // we don't care about the patch version and it's a bit of a pain to parse
}

impl Ord for RustVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.major.cmp(&other.major) {
            Ordering::Equal => self.minor.cmp(&other.minor),
            other => other,
        }
    }
}

impl FromStr for RustVersion {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s
            .strip_prefix("rustc ")
            .ok_or("Rust version string does not start with 'rustc'!")?;
        let mut iter = s.split('.');
        let major: u32 = iter
            .next()
            .ok_or("No major version found in `rustc --version` output!")?
            .parse()
            .map_err(|_| {
                "Failed to parse major version in `rustc --version` output as a number!"
            })?;
        let minor: u32 = iter
            .next()
            .ok_or("No minor version found in `rustc --version` output!")?
            .parse()
            .map_err(|_| {
                "Failed to parse minor version in `rustc --version` output as a number!"
            })?;
        Ok(RustVersion { major, minor })
    }
}

/// Checks whether the compiler supports sanitizers on stable channel.
/// Such compilers (even nightly) do not support `-Zsanitizer` flag,
/// and require a different combination of flags even on nightly.
///
/// Stabilization PR with more info: <https://github.com/rust-lang/rust/pull/123617>
impl RustVersion {
    pub fn has_sanitizers_on_stable(&self) -> bool {
        // TODO: the release that stabilizes sanitizers is not currently known.
        // This value is a PLACEHOLDER.
        let release_that_stabilized_sanitizers = RustVersion {
            major: 1,
            minor: 85,
        };
        self >= &release_that_stabilized_sanitizers
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parsing_stable() {
        let version_string = "rustc 1.78.0 (9b00956e5 2024-04-29)";
        let result = RustVersion::from_str(version_string).unwrap();
        assert_eq!(
            result,
            RustVersion {
                major: 1,
                minor: 78
            }
        );
        assert!(!is_nightly(version_string))
    }

    #[test]
    fn test_parsing_nightly() {
        let version_string = "rustc 1.81.0-nightly (d7f6ebace 2024-06-16)";
        let result = RustVersion::from_str(version_string).unwrap();
        assert_eq!(
            result,
            RustVersion {
                major: 1,
                minor: 81
            }
        );
        assert!(is_nightly(version_string))
    }

    #[test]
    fn test_parsing_future_stable() {
        let version_string = "rustc 2.356.1 (deadfaced 2029-04-01)";
        let result = RustVersion::from_str(version_string).unwrap();
        assert_eq!(
            result,
            RustVersion {
                major: 2,
                minor: 356
            }
        );
        assert!(!is_nightly(version_string))
    }
}
