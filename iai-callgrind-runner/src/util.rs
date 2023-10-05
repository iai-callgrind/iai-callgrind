//! This module provides common utility functions
use std::ffi::OsStr;
use std::io::{self, stdin, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, Context, Result};
use log::{debug, log_enabled, trace, Level};
use which::which;

use crate::error::Error;

/// Method to read, decode and deserialize the data sent by iai-callgrind
///
/// iai-callgrind uses elements from the [`crate::api`], so the runner can understand which elements
/// can be received by this method
pub fn receive_benchmark<T>(num_bytes: usize) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let mut encoded = vec![];
    let mut stdin = stdin();
    stdin
        .read_to_end(&mut encoded)
        .with_context(|| "Failed to read encoded configuration")?;
    assert!(
        encoded.len() == num_bytes,
        "Bytes mismatch when decoding configuration: Expected {num_bytes} bytes but received: {} \
         bytes",
        encoded.len()
    );

    let benchmark: T =
        bincode::deserialize(&encoded).with_context(|| "Failed to decode configuration")?;

    Ok(benchmark)
}

/// Convert a boolean value to a `yes` or `no` string
pub fn bool_to_yesno(value: bool) -> String {
    if value {
        "yes".to_owned()
    } else {
        "no".to_owned()
    }
}

/// Convert a `yes` or `no` string to a boolean value
///
/// This method is the counterpart to [`bool_to_yesno`] and can fail if the string doesn't match
/// exactly.
pub fn yesno_to_bool(value: &str) -> Option<bool> {
    match value.trim() {
        "yes" => Some(true),
        "no" => Some(false),
        _ => None,
    }
}

/// Truncate a utf-8 [`std::str`] to a give `len`
pub fn truncate_str_utf8(string: &str, len: usize) -> &str {
    if let Some((pos, c)) = string
        .char_indices()
        .take_while(|(i, c)| i + c.len_utf8() <= len)
        .last()
    {
        &string[..pos + c.len_utf8()]
    } else {
        &string[..0]
    }
}

/// Trim a slice of `u8` from ascii whitespace
pub fn trim(bytes: &[u8]) -> &[u8] {
    let from = match bytes.iter().position(|x| !x.is_ascii_whitespace()) {
        Some(i) => i,
        None => return &bytes[0..0],
    };
    let to = bytes
        .iter()
        .rposition(|x| !x.is_ascii_whitespace())
        .unwrap();
    &bytes[from..=to]
}

/// Dump all bytes data to `stdout`
pub fn write_all_to_stdout(bytes: &[u8]) {
    if !bytes.is_empty() {
        let stdout = io::stdout();
        let stdout = stdout.lock();
        let mut writer = BufWriter::new(stdout);
        writer
            .write_all(bytes)
            .and_then(|_| writer.flush())
            .unwrap();
        if !bytes.last().map_or(false, |l| *l == b'\n') {
            println!();
        }
    }
}

/// Dump all bytes data to `stderr`
pub fn write_all_to_stderr(bytes: &[u8]) {
    if !bytes.is_empty() {
        let stderr = io::stderr();
        let stderr = stderr.lock();
        let mut writer = BufWriter::new(stderr);
        writer
            .write_all(bytes)
            .and_then(|_| writer.flush())
            .unwrap();
        if !bytes.last().map_or(false, |l| *l == b'\n') {
            eprintln!();
        }
    }
}

/// Copy a directory from `source` to `dest`
///
/// If `follow_symlinks` is true copy the symlinked file or directory instead of the symlink itself
pub fn copy_directory(source: &Path, dest: &Path, follow_symlinks: bool) -> Result<()> {
    let cp = resolve_binary_path("cp")?;
    let mut command = Command::new(&cp);
    if follow_symlinks {
        command.args(["-H", "--dereference"]);
    }
    command.args([
        "--verbose",
        "--recursive",
        "--preserve=mode,ownership,timestamps",
    ]);
    command.arg(source);
    command.arg(dest);
    let (stdout, stderr) = command
        .output()
        .map_err(|error| Error::LaunchError(cp, error.to_string()))
        .and_then(|output| {
            if output.status.success() {
                Ok((output.stdout, output.stderr))
            } else {
                Err(Error::BenchmarkLaunchError(output))
            }
        })?;

    if !stdout.is_empty() {
        trace!("copy fixtures: stdout:");
        if log_enabled!(Level::Trace) {
            write_all_to_stderr(&stdout);
        }
    }
    if !stderr.is_empty() {
        trace!("copy fixtures: stderr:");
        if log_enabled!(Level::Trace) {
            write_all_to_stderr(&stderr);
        }
    }
    Ok(())
}

/// Try to resolve the absolute path of a binary from the `PATH` and relative paths
///
/// If the binary is a name without path separators the PATH is tried, otherwise if not absolute
/// a relative path is tried. If the path is already absolute checks if it is executable.
pub fn resolve_binary_path<T>(binary: T) -> Result<PathBuf>
where
    T: AsRef<OsStr>,
{
    let binary = binary.as_ref();
    match which(binary) {
        Ok(path) => {
            debug!("Found '{}': '{}'", binary.to_string_lossy(), path.display());
            Ok(path)
        }
        Err(error) => Err(
            anyhow! {"{error}: '{0}' could not be found. Is '{0}' installed, executable and in the PATH?",
                binary.to_string_lossy()
            },
        ),
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::empty_0("", 0, "")]
    #[case::empty_1("", 1, "")]
    #[case::single_0("a", 0, "")]
    #[case::single_1("a", 1, "a")]
    #[case::single_2("a", 2, "a")]
    #[case::two_0("ab", 0, "")]
    #[case::two_1("ab", 1, "a")]
    #[case::two_2("ab", 2, "ab")]
    #[case::two_3("ab", 3, "ab")]
    #[case::two_usize_max("ab", usize::MAX, "ab")]
    #[case::hundred_0(&"a".repeat(100), 0, "")]
    #[case::hundred_99(&"ab".repeat(50), 99, &"ab".repeat(50)[..99])]
    #[case::hundred_100(&"a".repeat(100), 100, &"a".repeat(100))]
    #[case::hundred_255(&"a".repeat(100), 255, &"a".repeat(100))]
    #[case::multi_byte_0("µ", 0, "")]
    #[case::multi_byte_1("µ", 1, "")]
    #[case::multi_byte_2("µ", 2, "µ")]
    #[case::multi_byte_3("µ", 3, "µ")]
    #[case::uni_then_multi_byte_0("aµ", 0, "")]
    #[case::uni_then_multi_byte_1("aµ", 1, "a")]
    #[case::uni_then_multi_byte_2("aµ", 2, "a")]
    #[case::uni_then_multi_byte_3("aµ", 3, "aµ")]
    #[case::uni_then_multi_byte_4("aµ", 4, "aµ")]
    #[case::multi_byte_then_uni_0("µa", 0, "")]
    #[case::multi_byte_then_uni_1("µa", 1, "")]
    #[case::multi_byte_then_uni_2("µa", 2, "µ")]
    #[case::multi_byte_then_uni_3("µa", 3, "µa")]
    #[case::multi_byte_then_uni_4("µa", 4, "µa")]
    fn test_truncate_str(#[case] input: &str, #[case] len: usize, #[case] expected: &str) {
        assert_eq!(truncate_str_utf8(input, len), expected);
    }
}
